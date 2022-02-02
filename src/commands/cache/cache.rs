use log::info;
use momento::{cache::CacheClient, sdk::Momento};

use crate::error::CliError;

async fn get_momento_instance(auth_token: String) -> Result<Momento, CliError> {
    match Momento::new(auth_token).await {
        Ok(m) => Ok(m),
        Err(e) => Err(CliError { msg: e.to_string() }),
    }
}

async fn get_momento_cache(
    cache_name: String,
    auth_token: String,
) -> Result<CacheClient, CliError> {
    let mut momento = get_momento_instance(auth_token).await?;
    match momento.get_cache(&cache_name, 100).await {
        Ok(c) => Ok(c),
        Err(e) => Err(CliError { msg: e.to_string() }),
    }
}

pub async fn create_cache(cache_name: String, auth_token: String) -> Result<(), CliError> {
    info!("create cache called");
    let mut momento = get_momento_instance(auth_token).await?;
    match momento.create_cache(&cache_name).await {
        Ok(_) => info!("created cache {}", cache_name),
        Err(e) => return Err(CliError { msg: e.to_string() }),
    };
    Ok(())
}

pub async fn delete_cache(cache_name: String, auth_token: String) -> Result<(), CliError> {
    info!("delete cache called");
    let mut momento = get_momento_instance(auth_token).await?;
    match momento.delete_cache(&cache_name).await {
        Ok(_) => info!("deleted cache {}", cache_name),
        Err(e) => return Err(CliError { msg: e.to_string() }),
    };
    Ok(())
}

pub async fn list_caches(auth_token: String) -> Result<(), CliError> {
    info!("list cache called");
    let mut momento = get_momento_instance(auth_token).await?;
    match momento.list_caches(None).await {
        Ok(res) => {
            res.caches
                .into_iter()
                .for_each(|cache| println!("{}", cache.cache_name));
        }
        Err(e) => return Err(CliError { msg: e.to_string() }),
    }
    Ok(())
}

pub async fn set(
    cache_name: String,
    auth_token: String,
    key: String,
    value: String,
    ttl_seconds: u32,
) -> Result<(), CliError> {
    info!("setting key: {} into cache: {}", key, cache_name);
    let cache = get_momento_cache(cache_name, auth_token).await?;
    match cache.set(key, value, Some(ttl_seconds)).await {
        Ok(_) => info!("set success"),
        Err(e) => return Err(CliError { msg: e.to_string() }),
    };
    Ok(())
}

pub async fn get(cache_name: String, auth_token: String, key: String) -> Result<(), CliError> {
    info!("getting key: {} from cache: {}", key, cache_name);
    let cache = get_momento_cache(cache_name, auth_token).await?;

    match cache.get(key).await {
        Ok(r) => {
            if matches!(
                r.result,
                momento::response::cache_get_response::MomentoGetStatus::HIT
            ) {
                println!("{}", r.as_string());
            } else {
                info!("cache miss");
            }
        }
        Err(e) => {
            return Err(CliError {
                msg: format!("failed to get from cache: {}", e),
            })
        }
    };
    Ok(())
}
