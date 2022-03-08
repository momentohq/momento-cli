use log::{debug, info};
use serde::{de::DeserializeOwned, Serialize};

use tokio::fs;

use crate::{
    commands::cache::cache::create_cache,
    config::{Config, Credentials, Profiles},
    error::CliError,
    utils::{
        file::{
            create_file_if_not_exists, get_config_file_path, get_credentials_file_path,
            get_momento_dir, prompt_user_for_input, read_toml_file, set_file_read_write,
            set_file_readonly, write_to_existing_file,
        },
        user::{get_config_for_profile, get_creds_for_profile},
    },
};

pub async fn configure_momento(profile_name: &str) -> Result<(), CliError> {
    let credentials = prompt_user_for_creds(profile_name).await?;
    let config = prompt_user_for_config(profile_name).await?;

    let momento_dir = get_momento_dir();
    let credentials_file_path = get_credentials_file_path();
    let config_file_path = get_config_file_path();

    match fs::create_dir_all(momento_dir).await {
        Ok(_) => (),
        Err(e) => {
            return Err(CliError {
                msg: format!("failed to create directory: {}", e),
            })
        }
    };
    create_file_if_not_exists(&credentials_file_path).await?;
    create_file_if_not_exists(&config_file_path).await?;

    // explicitly allowing read/write access to the credentials file
    set_file_read_write(&credentials_file_path).await?;
    add_profile(profile_name, credentials.clone(), &credentials_file_path).await?;
    // explicitly revoking that access
    set_file_readonly(&credentials_file_path).await?;

    add_profile(profile_name, config.clone(), &config_file_path).await?;

    match create_cache(config.cache, credentials.token).await{
        Ok(_) => (),
        Err(e) => if e.msg.contains("already exists") {
            info!("default cache already exists");
            ()
        } else {
            return Err(e)
        },
    };

    Ok(())
}

async fn prompt_user_for_creds(profile_name: &str) -> Result<Credentials, CliError> {
    let current_credentials = get_creds_for_profile(profile_name)
        .await
        .unwrap_or_default();

    let token = prompt_user_for_input("Token", current_credentials.token.as_str(), true).await?;

    return Ok(Credentials { token });
}

async fn prompt_user_for_config(profile_name: &str) -> Result<Config, CliError> {
    let current_config = get_config_for_profile(profile_name)
        .await
        .unwrap_or_default();

    let cache_name =
        prompt_user_for_input("Default Cache", current_config.cache.as_str(), false).await?;
    let prompt_ttl = if current_config.ttl == 0 {
        600
    } else {
        current_config.ttl
    };
    let ttl = match prompt_user_for_input(
        "Default Ttl Seconds",
        prompt_ttl.to_string().as_str(),
        false,
    )
    .await?
    .parse::<u32>()
    {
        Ok(ttl) => ttl,
        Err(e) => {
            return Err(CliError {
                msg: format!("failed to parse ttl: {}", e),
            })
        }
    };

    return Ok(Config {
        cache: cache_name,
        ttl,
    });
}

async fn add_profile<T>(
    profile_name: &str,
    config: T,
    config_file_path: &str,
) -> Result<(), CliError>
where
    T: DeserializeOwned + Default + Serialize,
{
    let mut toml = match read_toml_file::<Profiles<T>>(config_file_path).await {
        Ok(t) => t,
        Err(_) => {
            debug!("config file is invalid, most likely we are creating it for the first time. Overwriting it with new profile");
            Profiles::<T>::default()
        }
    };
    toml.profile.insert(profile_name.to_string(), config);
    let new_profile_string = toml::to_string(&toml).unwrap();
    write_to_existing_file(config_file_path, &new_profile_string).await?;
    Ok(())
}
