use std::env;

use crate::{
    config::{Config, Credentials, Profiles},
    error::CliError,
    utils::file::{get_config_file_path, get_credentials_file_path, read_toml_file},
};

pub async fn get_creds_and_config() -> Result<(Credentials, Config), CliError> {
    let profile = get_profile_to_use();
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

pub fn get_profile_to_use() -> String {
    env::var("MOMENTO_PROFILE").unwrap_or("default".to_string())
}

pub async fn get_creds_for_profile(profile: &str) -> Result<Credentials, CliError> {
    let path = get_credentials_file_path();
    let credentials_toml = match read_toml_file::<Profiles<Credentials>>(&path).await {
        Ok(c) => c,
        Err(_) => {
            return Err(CliError {
                msg: format!(
                "failed to read credentials, please run 'momento configure' to setup credentials"
            ),
            })
        }
    };

    let creds_result = match credentials_toml.profile.get(profile) {
        Some(c) => c,
        None => return Err(CliError{
            msg: format!("failed to get credentials for profile {}, please run 'momento configure' to confiure your profile", profile)
        }),
    };

    return Ok(creds_result.clone());
}

pub async fn get_config_for_profile(profile: &str) -> Result<Config, CliError> {
    let path = get_config_file_path();
    let profile_toml = match read_toml_file::<Profiles<Config>>(&path).await {
        Ok(c) => c,
        Err(e) => return Err(CliError{
            msg: format!("failed to read profile '{}', please run 'momento configure' to setup your profile, {:#?}", profile, e)
    }),
    };

    let config_result = match profile_toml.profile.get(profile) {
        Some(c) => c,
        None => {
            return Err(CliError {
                msg: format!(
            "failed to read profile {}, please run 'momento configure' to confiure your profile",
            profile
        ),
            })
        }
    };

    return Ok(config_result.clone());
}
