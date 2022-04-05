use log::{debug, info};
use momento::simple_cache_client::SimpleCacheClient;

use crate::error::CliError;

async fn get_momento_instance(auth_token: String) -> Result<SimpleCacheClient, CliError> {
    match SimpleCacheClient::new(auth_token, 100).await {
        Ok(m) => Ok(m),
        Err(e) => Err(CliError { msg: e.to_string() }),
    }
}

pub async fn create_cache(cache_name: String, auth_token: String) -> Result<(), CliError> {
    debug!("creating cache...");
    let mut momento = get_momento_instance(auth_token).await?;
    match momento.create_cache(&cache_name).await {
        Ok(_) => (),
        Err(e) => return Err(CliError { msg: e.to_string() }),
    };
    Ok(())
}

pub async fn delete_cache(cache_name: String, auth_token: String) -> Result<(), CliError> {
    debug!("deleting cache...");
    let mut momento = get_momento_instance(auth_token).await?;
    match momento.delete_cache(&cache_name).await {
        Ok(_) => (),
        Err(e) => return Err(CliError { msg: e.to_string() }),
    };
    Ok(())
}

pub async fn list_caches(auth_token: String) -> Result<(), CliError> {
    debug!("list cache called");
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
    ttl_seconds: u64,
) -> Result<(), CliError> {
    debug!("setting key: {} into cache: {}", key, cache_name);
    let mut momento = get_momento_instance(auth_token).await?;
    match momento
        .set(&cache_name, key, value, Some(ttl_seconds))
        .await
    {
        Ok(_) => debug!("set success"),
        Err(e) => return Err(CliError { msg: e.to_string() }),
    };
    Ok(())
}

pub async fn get(cache_name: String, auth_token: String, key: String) -> Result<(), CliError> {
    debug!("getting key: {} from cache: {}", key, cache_name);
    let mut momento = get_momento_instance(auth_token).await?;
    match momento.get(&cache_name, key).await {
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
