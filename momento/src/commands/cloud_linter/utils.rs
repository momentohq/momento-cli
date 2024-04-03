use crate::error::CliError;
use aws_sdk_dynamodb::error::{DisplayErrorContext, SdkError};
use governor::DefaultDirectRateLimiter;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

pub(crate) async fn rate_limit<F, Fut>(
    limiter: Arc<DefaultDirectRateLimiter>,
    operation: F,
) -> Fut::Output
where
    F: FnOnce() -> Fut,
    Fut: Future,
{
    loop {
        let permit = limiter.check();
        match permit {
            Ok(_) => {
                return operation().await;
            }
            Err(_) => {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }
}

impl<E> From<SdkError<E>> for CliError
where
    E: std::fmt::Display + std::error::Error + 'static,
{
    fn from(err: SdkError<E>) -> Self {
        let display_err = DisplayErrorContext(err);
        CliError {
            msg: format!("{display_err:?}"),
        }
    }
}

impl From<serde_json::Error> for CliError {
    fn from(val: serde_json::Error) -> Self {
        CliError {
            msg: format!("{val:?}"),
        }
    }
}

impl From<std::io::Error> for CliError {
    fn from(val: std::io::Error) -> Self {
        CliError {
            msg: format!("{val:?}"),
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

        let quota = Quota::per_second(
            core::num::NonZeroU32::new(10).expect("should create non-zero quota"),
        );
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
                let _ = rate_limit(limiter, func).await;
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
