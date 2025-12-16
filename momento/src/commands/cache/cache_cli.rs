use log::debug;
use momento::cache::{GetResponse, SetRequest};
use std::process::exit;
use std::time::Duration;

use crate::{
    config::Credentials,
    error::CliError,
    utils::{
        client::{get_momento_client, interact_with_momento},
        console::console_data,
    },
};

pub async fn create_cache(
    cache_name: String,
    credentials: Credentials,
    endpoint: Option<String>,
) -> Result<(), CliError> {
    let client = get_momento_client(credentials, endpoint).await?;

    interact_with_momento("creating cache...", client.create_cache(&cache_name))
        .await
        .map(|_| ())
}

pub async fn delete_cache(
    cache_name: String,
    credentials: Credentials,
    endpoint: Option<String>,
) -> Result<(), CliError> {
    let client = get_momento_client(credentials, endpoint).await?;

    interact_with_momento("deleting cache...", client.delete_cache(&cache_name))
        .await
        .map(|_| ())
}

pub async fn list_caches(
    credentials: Credentials,
    endpoint: Option<String>,
) -> Result<(), CliError> {
    let client = get_momento_client(credentials, endpoint).await?;

    let list_result = interact_with_momento("listing caches...", client.list_caches()).await?;

    list_result
        .caches
        .into_iter()
        .for_each(|cache| console_data!("{}", cache.name));

    Ok(())
}

pub async fn flush_cache(
    cache_name: String,
    credentials: Credentials,
    endpoint: Option<String>,
) -> Result<(), CliError> {
    let client = get_momento_client(credentials, endpoint).await?;
    client.flush_cache(&cache_name).await?;
    Ok(())
}

pub async fn set(
    cache_name: String,
    credentials: Credentials,
    key: String,
    value: String,
    ttl_seconds: u64,
    endpoint: Option<String>,
) -> Result<(), CliError> {
    debug!("setting key: {} into cache: {}", key, cache_name);
    let client = get_momento_client(credentials, endpoint).await?;

    let set_request = SetRequest::new(cache_name, key, value).ttl(Duration::from_secs(ttl_seconds));
    interact_with_momento("setting...", client.send_request(set_request))
        .await
        .map(|_| ())
}

pub async fn get(
    cache_name: String,
    credentials: Credentials,
    key: String,
    endpoint: Option<String>,
) -> Result<(), CliError> {
    debug!("getting key: {} from cache: {}", key, cache_name);

    let client = get_momento_client(credentials, endpoint).await?;

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
    cache_name: String,
    credentials: Credentials,
    key: String,
    endpoint: Option<String>,
) -> Result<(), CliError> {
    debug!("deleting key: {} from cache: {}", key, cache_name);

    let client = get_momento_client(credentials, endpoint).await?;

    interact_with_momento("deleting...", client.delete(&cache_name, key))
        .await
        .map(|_| ())
}
