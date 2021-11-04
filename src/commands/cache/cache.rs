use std::panic;

use log::info;
use momento::{cache::CacheClient, sdk::Momento};

async fn get_momento_instance(auth_token: String) -> Momento {
    match Momento::new(auth_token).await {
        Ok(m) => m,
        Err(e) => panic!("{}", e)    
    }
}

async fn get_momento_cache(cache_name: String, auth_token: String) -> CacheClient {
    let mut momento = get_momento_instance(auth_token).await;
    match momento.get_cache(&cache_name, 100).await {
        Ok(c) => c,
        Err(e) => panic!("{}", e),
    }
}

pub async fn create_cache(cache_name: String, auth_token: String) {
    info!("create cache called");
    let mut momento = get_momento_instance(auth_token).await;
    match momento.create_cache(&cache_name).await {
        Ok(_) => info!("created cache {}", cache_name),
        Err(e) => panic!("{}", e),
    }
}

pub async fn delete_cache(cache_name: String, auth_token: String) {
    info!("delete cache called");
    let mut momento = get_momento_instance(auth_token).await;
    match momento.delete_cache(&cache_name).await {
        Ok(_) => info!("deleted cache {}", cache_name),
        Err(e) => panic!("{}", e),
    }
}

pub async fn set(
    cache_name: String,
    auth_token: String,
    key: String,
    value: String,
    ttl_seconds: u32,
) {
    info!("setting key: {} into cache: {}", key, cache_name);
    let cache = get_momento_cache(cache_name, auth_token).await;
    match cache.set(key, value, Some(ttl_seconds)).await {
        Ok(_) => info!("set success"),
        Err(e) => panic!("{}", e),
    }
}

pub async fn get(cache_name: String, auth_token: String, key: String) {
    info!("getting key: {} from cache: {}", key, cache_name);
    let cache = get_momento_cache(cache_name, auth_token).await;

    match cache.get(key).await {
        Ok(r) => {
            if matches!(r.result, momento::response::cache_get_response::MomentoGetStatus::HIT) {
                println!("{}", r.as_string());
            } else {
                info!("cache miss");
            }
        },
        Err(e) => panic!("failed to get from cache, error: {}", e),
    };
}
