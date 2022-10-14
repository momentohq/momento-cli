use log::debug;
use std::num::NonZeroU64;
use std::process::exit;

use crate::{
    error::CliError,
    utils::{
        client::{get_momento_client, interact_with_momento},
        console::console_data,
    },
};

pub async fn create_cache(
    cache_name: String,
    auth_token: String,
    endpoint: Option<String>,
) -> Result<(), CliError> {
    let mut client = get_momento_client(auth_token, endpoint).await?;

    interact_with_momento("creating cache...", client.create_cache(&cache_name)).await
}

pub async fn delete_cache(
    cache_name: String,
    auth_token: String,
    endpoint: Option<String>,
) -> Result<(), CliError> {
    let mut client = get_momento_client(auth_token, endpoint).await?;

    interact_with_momento("deleting cache...", client.delete_cache(&cache_name)).await
}

pub async fn list_caches(auth_token: String, endpoint: Option<String>) -> Result<(), CliError> {
    let mut client = get_momento_client(auth_token, endpoint).await?;

    let list_result = interact_with_momento("listing caches...", client.list_caches(None)).await?;

    list_result
        .caches
        .into_iter()
        .for_each(|cache| console_data!("{}", cache.cache_name));

    Ok(())
}

pub async fn set(
    cache_name: String,
    auth_token: String,
    key: String,
    value: String,
    ttl_seconds: u64,
    endpoint: Option<String>,
) -> Result<(), CliError> {
    debug!("setting key: {} into cache: {}", key, cache_name);
    let mut client = get_momento_client(auth_token, endpoint).await?;

    interact_with_momento(
        "setting...",
        client.set(
            &cache_name,
            key,
            value,
            Some(NonZeroU64::new(ttl_seconds).unwrap()),
        ),
    )
    .await
    .map(|_| ())
}

pub async fn get(
    cache_name: String,
    auth_token: String,
    key: String,
    endpoint: Option<String>,
) -> Result<(), CliError> {
    debug!("getting key: {} from cache: {}", key, cache_name);

    let mut client = get_momento_client(auth_token, endpoint).await?;

    let response = interact_with_momento("getting...", client.get(&cache_name, key)).await?;
    match response.result {
        momento::response::cache_get_response::MomentoGetStatus::HIT => {
            console_data!("{}", response.as_string())
        }
        momento::response::cache_get_response::MomentoGetStatus::MISS => {
            debug!("cache miss");
            exit(1)
        }
        momento::response::cache_get_response::MomentoGetStatus::ERROR => {
            debug!("something terrible happened with the wire protocol");
            exit(13)
        }
    };
    Ok(())
}
