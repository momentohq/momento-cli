use crate::config::DEFAULT_CACHE_NAME;
use crate::utils::file::create_necessary_files;

use crate::utils::user::{prompt_user_for_input, update_credentials, update_profile};
use crate::{
    commands::cache::cache_cli::create_cache,
    config::{Config, Credentials},
    error::CliError,
    utils::{
        console::console_info,
        user::{get_config_for_profile, get_credentials_for_profile},
    },
};

pub async fn configure_momento(quick: bool, profile_name: &str) -> Result<(), CliError> {
    create_necessary_files().await?;
    let credentials = prompt_user_for_creds(profile_name).await?;
    let config = prompt_user_for_config(quick, profile_name).await?;

    update_profile(profile_name, &config).await?;
    update_credentials(profile_name, &credentials).await?;

    // TODO: Update the endpoint to read from config
    match create_cache(config.cache.clone(), credentials.token, None).await {
        Ok(_) => console_info!(
            "{} successfully created as the default with default TTL of {}s",
            config.cache.clone(),
            config.ttl
        ),
        Err(e) => {
            if e.msg.contains("already exists") {
                // Nothing to do here; the cache already exists but users won't find that particularly
                // interesting.
            } else {
                return Err(e);
            }
        }
    };
    Ok(())
}

async fn prompt_user_for_creds(profile_name: &str) -> Result<Credentials, CliError> {
    let current_credentials = get_credentials_for_profile(profile_name)
        .await
        .unwrap_or_default();

    console_info!("Please paste your Momento auth token.  (If you do not have an auth token, use `momento account` to generate one.)");
    console_info!(
        "Windows users: if CTRL-V does not work, try right-click or SHIFT-INSERT to paste."
    );
    console_info!("");

    let token = prompt_user_for_input("Token", current_credentials.token.as_str(), true).await?;

    // todo: we should remove forever valid tokens once login feature is released
    Ok(Credentials::valid_forever(token))
}

async fn prompt_user_for_config(quick: bool, profile_name: &str) -> Result<Config, CliError> {
    let current_config = get_config_for_profile(profile_name)
        .await
        .unwrap_or_default();

    let prompt_cache = if current_config.cache.is_empty() {
        DEFAULT_CACHE_NAME
    } else {
        current_config.cache.as_str()
    };
    let mut cache_name = prompt_cache.to_string();
    if !quick {
        cache_name = match prompt_user_for_input("Default Cache", prompt_cache, false).await {
            Ok(s) => s,
            Err(e) => return Err(e),
        };
    }
    let cache_name_to_use = if cache_name.is_empty() {
        DEFAULT_CACHE_NAME.to_string()
    } else {
        cache_name
    };
    let prompt_ttl = if current_config.ttl == 0 {
        600
    } else {
        current_config.ttl
    };
    let mut ttl = prompt_ttl;
    if !quick {
        ttl = match prompt_user_for_input(
            "Default Ttl Seconds",
            prompt_ttl.to_string().as_str(),
            false,
        )
        .await?
        .parse::<u64>()
        {
            Ok(ttl) => ttl,
            Err(e) => {
                return Err(CliError {
                    msg: format!("failed to parse ttl: {e}"),
                })
            }
        };
    }

    Ok(Config {
        cache: cache_name_to_use,
        ttl,
    })
}
