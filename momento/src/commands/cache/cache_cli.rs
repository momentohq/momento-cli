use log::debug;
use momento::cache::{CacheClient, GetResponse, SetRequest};
use std::process::exit;
use std::time::Duration;

use crate::{
    error::CliError,
    utils::{client::interact_with_momento, console::console_data},
};

pub async fn create_cache(client: CacheClient, cache_name: String) -> Result<(), CliError> {
    interact_with_momento("creating cache...", client.create_cache(&cache_name))
        .await
        .map(|_| ())
}

pub async fn delete_cache(client: CacheClient, cache_name: String) -> Result<(), CliError> {
    interact_with_momento("deleting cache...", client.delete_cache(&cache_name))
        .await
        .map(|_| ())
}

pub async fn list_caches(client: CacheClient) -> Result<(), CliError> {
    let list_result = interact_with_momento("listing caches...", client.list_caches()).await?;

    list_result
        .caches
        .into_iter()
        .for_each(|cache| console_data!("{}", cache.name));

    Ok(())
}

pub async fn flush_cache(client: CacheClient, cache_name: String) -> Result<(), CliError> {
    client.flush_cache(&cache_name).await?;
    Ok(())
}

pub async fn set(
    client: CacheClient,
    cache_name: String,
    key: String,
    value: String,
    ttl_seconds: u64,
) -> Result<(), CliError> {
    debug!("setting key: {} into cache: {}", key, cache_name);
    let set_request = SetRequest::new(cache_name, key, value).ttl(Duration::from_secs(ttl_seconds));
    interact_with_momento("setting...", client.send_request(set_request))
        .await
        .map(|_| ())
}

pub async fn get(client: CacheClient, cache_name: String, key: String) -> Result<(), CliError> {
    debug!("getting key: {} from cache: {}", key, cache_name);

    let response = interact_with_momento("getting...", client.get(&cache_name, key)).await?;
    match response {
        GetResponse::Hit { value } => {
            let value: String = value.try_into()?;
            console_data!("{}", value);
        }
        GetResponse::Miss => {
            debug!("cache miss");
            exit(1)
        }
    };
    Ok(())
}

pub async fn delete_key(
    client: CacheClient,
    cache_name: String,
    key: String,
) -> Result<(), CliError> {
    debug!("deleting key: {} from cache: {}", key, cache_name);
    interact_with_momento("deleting...", client.delete(&cache_name, key))
        .await
        .map(|_| ())
}
