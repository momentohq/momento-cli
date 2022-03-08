use crate::{
    config::{Config, Credentials},
    error::CliError,
    utils::file::{get_config_file_path, get_credentials_file_path, read_file},
};

pub async fn get_creds_and_config(profile: &str) -> Result<(Credentials, Config), CliError> {
    let creds = match get_creds_for_profile(&profile).await {
        Ok(c) => c,
        Err(e) => return Err(e),
    };
    let config = match get_config_for_profile(&profile).await {
        Ok(c) => c,
        Err(e) => return Err(e),
    };

    return Ok((creds, config));
}

pub async fn get_creds_for_profile(profile: &str) -> Result<Credentials, CliError> {
    let path = get_credentials_file_path();
    let credentials = match read_file(&path).await {
        Ok(c) => c,
        Err(_) => {
            return Err(CliError {
                msg: format!(
                "failed to read credentials, please run 'momento configure' to setup credentials"
            ),
            })
        }
    };

    let creds_result = match credentials.get(profile, "token") {
        Some(c) => c,
        None => return Err(CliError{
            msg: format!("failed to get credentials for profile {}, please run 'momento configure' to configure your profile", profile)
        }),
    };

    return Ok(Credentials {
        token: creds_result,
    });
}

pub async fn get_config_for_profile(profile: &str) -> Result<Config, CliError> {
    let path = get_config_file_path();
    let configs = match read_file(&path).await {
        Ok(c) => c,
        Err(_) => {
            return Err(CliError {
                msg: format!(
                "failed to read credentials, please run 'momento configure' to setup credentials"
            ),
            })
        }
    };

    let cache_result = match configs.get(profile, "cache") {
        Some(c) => c,
        None => return Err(CliError{
            msg: format!("failed to get cache config for profile {}, please run 'momento configure' to configure your profile", profile)
        }),
    };

    let ttl_result = match configs.get(profile, "ttl") {
        Some(c) => c,
        None => return Err(CliError{
            msg: format!("failed to get ttl config for profile {}, please run 'momento configure' to configure your profile", profile)
        }),
    };

    return Ok(Config {
        cache: cache_result,
        ttl: ttl_result.parse::<u32>().unwrap(),
    });
}
