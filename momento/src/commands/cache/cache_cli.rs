use log::debug;
use momento::cache::{CacheClient, GetResponse, SetRequest};
use std::process::exit;
use std::time::Duration;

use super::utils::write_default_config;
use crate::{
    config::Config,
    error::CliError,
    utils::{
        client::interact_with_momento, console::console_data, file::prompt_user_for_input,
        user::get_config_for_profile,
    },
};

pub async fn create_cache(client: CacheClient, cache_name: String) -> Result<(), CliError> {
    interact_with_momento("creating cache...", client.create_cache(&cache_name))
        .await
        .map(|_| ())
}

pub async fn set_default_cache(
    client: CacheClient,
    profile_name: &str,
    new_cache_name: String,
    new_ttl_seconds: u64,
) -> Result<(), CliError> {
    if let Ok(config) = get_config_for_profile(profile_name).await {
        if config.cache != new_cache_name {
            let confirmation = prompt_user_for_input(
                format!(
                    "Your profile \"{profile_name}\" already has default Serverless Cache \"{}\". \
                     Are you sure you want to replace it?",
                    config.cache
                )
                .as_str(),
                "n",
                false,
            )
            .await?;
            if confirmation.to_lowercase() != "y" && confirmation.to_lowercase() != "yes" {
                console_data!("Keeping your existing default.");
                return Ok(());
            }
        }
    }

    let found = match interact_with_momento("listing caches...", client.list_caches()).await {
        Ok(list_result) => list_result
            .caches
            .into_iter()
            .any(|cache| cache.name == new_cache_name),
        Err(_) => false,
    };
    if !found {
        create_cache(client, new_cache_name.clone()).await?;
    }

    write_default_config(
        profile_name,
        Config {
            cache: new_cache_name.clone(),
            ttl: new_ttl_seconds,
        },
    )
    .await?;

    console_data!(
        "Your profile \"{profile_name}\" now has:\n\
         - Default Serverless Cache: {new_cache_name}\n\
         - Default TTL: {new_ttl_seconds} seconds"
    );
    Ok(())
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
