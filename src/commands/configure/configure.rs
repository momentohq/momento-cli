use configparser::ini::Ini;
use log::info;
use tokio::fs;

use crate::{
    commands::cache::cache::create_cache,
    config::{Config, Credentials},
    error::CliError,
    utils::{
        file::{
            create_file_if_not_exists, get_config_file_path, get_credentials_file_path,
            get_momento_dir, prompt_user_for_input, set_file_read_write, set_file_readonly,
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

    fs::create_dir_all(momento_dir).await.unwrap();
    match create_file_if_not_exists(&credentials_file_path, "credentials").await {
        Ok(_) => {
            // explicitly allowing read/write access to the credentials file
            set_file_read_write(&credentials_file_path).await.unwrap();
            add_profile_to_credentials(profile_name, credentials.clone(), &credentials_file_path)
                .await;
            // explicitly revoking that access
            set_file_readonly(&credentials_file_path).await.unwrap();

            match create_file_if_not_exists(&config_file_path, "config").await {
                Ok(_) => {
                    add_profile_to_config(profile_name, config.clone(), &config_file_path).await;
                    match create_cache(config.cache, credentials.token).await {
                        Ok(_) => info!("default cache successfully created"),
                        Err(e) => {
                            if e.msg.contains("already exists") {
                                info!("default cache already exists");
                                ()
                            } else {
                                return Err(e);
                            }
                        }
                    };
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
        Err(e) => {
            match create_file_if_not_exists(&config_file_path, "config").await {
                Ok(_) => {
                    add_profile_to_config(profile_name, config.clone(), &config_file_path).await;
                    match create_cache(config.cache, credentials.token).await {
                        Ok(_) => info!("default cache successfully created"),
                        Err(e) => {
                            if e.msg.contains("already exists") {
                                info!("default cache already exists");
                                ()
                            } else {
                                return Err(e);
                            }
                        }
                    };
                }
                Err(_) => {
                    return Err(CliError {
                        msg: format!("Existing credentials and config files detected.\nPlease edit $HOME/.momento/credentials and $HOME/.momento/config directly to add or modify profiles"),
                    });
                }
            }
            return Err(e);
        }
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

async fn add_profile_to_credentials(
    profile_name: &str,
    credentials: Credentials,
    credentials_file_path: &str,
) {
    let mut ini_map = Ini::new_cs();
    // Empty default_section for Ini instance so that "default" will be used as a section
    ini_map.set_default_section("");
    ini_map.set(profile_name, "token", Some(credentials.token));
    match ini_map.write(credentials_file_path) {
        Ok(_) => {}
        Err(_) => {}
    }
}

async fn add_profile_to_config(profile_name: &str, config: Config, config_file_path: &str) {
    let mut ini_map = Ini::new_cs();
    // Empty default_section for Ini instance so that "default" will be used as a section
    ini_map.set_default_section("");
    ini_map.set(profile_name, "cache", Some(config.cache));
    ini_map.set(profile_name, "ttl", Some(config.ttl.to_string()));
    match ini_map.write(config_file_path) {
        Ok(_) => {}
        Err(_) => {}
    }
}
