use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use governor::DefaultDirectRateLimiter;

#[allow(dead_code)] // remove after this is used outside a test
async fn rate_limit<F, Fut, T>(func: F, limiter: Arc<DefaultDirectRateLimiter>) -> T
    where
        F: Fn() -> Fut,
        Fut: Future<Output=T>,
{
    loop {
        let permit = limiter.check();
        match permit {
            Ok(_) => {
                return func().await;
            }
            Err(_) => {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use governor::{Quota, RateLimiter};
    use tokio::sync::Mutex;

    use super::*;

    #[tokio::test]
    async fn test_rate_limit() {
        let counter = Arc::new(Mutex::new(0));

        let quota =
            Quota::per_second(core::num::NonZeroU32::new(10).expect("should create non-zero quota"));
        let limiter = Arc::new(RateLimiter::direct(quota));

        let test_func = {
            let counter = Arc::clone(&counter);
            move || {
                let counter = Arc::clone(&counter);
                async move {
                    let mut count = counter.lock().await;
                    *count += 1;
                }
            }
        };
        let start_time = tokio::time::Instant::now();

        let mut tasks = Vec::new();
        for _ in 0..20 {
            let limiter = Arc::clone(&limiter);
            let func = test_func.clone();
            let task = tokio::spawn(async move {
                rate_limit(func, limiter).await;
            });
            tasks.push(task);
        }

        for task in tasks {
            task.await.expect("increment task should succeed");
        }

        let final_count = *counter.lock().await;
        assert_eq!(final_count, 20);

        let expected_duration = Duration::from_secs(1);
        assert!(start_time.elapsed() >= expected_duration);
    }
}
