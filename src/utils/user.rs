use chrono::{Duration, TimeZone, Utc};
use configparser::ini::Ini;

use crate::{
    config::{Config, Credentials},
    error::CliError,
    utils::file::{get_config_file_path, get_credentials_file_path, read_ini_file},
};

#[cfg(feature = "login")]
use crate::utils::file::write_to_file;

fn get_session_token(credentials: &Ini) -> Option<String> {
    let session_token = credentials.get(".momento_session", "token");
    if session_token.is_some() {
        let expiry = credentials
            .get(".momento_session", "valid_until")
            .map(|s| s.parse::<i64>().unwrap_or(0))
            .map(|expiry_timestamp| Utc.timestamp(expiry_timestamp, 0));
        if let Some(expiry_timestamp) = expiry {
            if Utc::now() + Duration::seconds(10) < expiry_timestamp {
                let expiring = expiry_timestamp - Utc::now();
                log::debug!("Found user session expiring in {}m", expiring.num_minutes());
                return session_token;
            }
            log::debug!("Token already expired at: {}", expiry_timestamp);
        } else {
            log::debug!(
                ".momento_session profile is missing the expiry time. Skipping this session..."
            );
        }
    }
    log::debug!("No session found in .momento_session profile...");
    None
}

#[cfg(feature = "login")]
fn set_session_token(credentials: &mut Ini, session_token: Option<String>, valid_for_seconds: u32) {
    let expiry_time = Utc::now() + Duration::seconds(valid_for_seconds.into());
    credentials.set(".momento_session", "token", session_token);
    credentials.set(
        ".momento_session",
        "valid_until",
        Some(expiry_time.timestamp().to_string()),
    );
}

#[cfg(feature = "login")]
pub async fn clobber_session_token(
    session_token: Option<String>,
    valid_for_seconds: u32,
) -> Result<(), CliError> {
    let mut credentials_file = read_credentials().await?;
    set_session_token(&mut credentials_file, session_token, valid_for_seconds);
    // TODO
    // TODO this is silly, should change write_to_file to take a String or have one fn for vec and one for string
    // TODO
    write_to_file(
        &get_credentials_file_path()?,
        credentials_file
            .writes()
            .split('\n')
            .map(|line| line.to_string())
            .collect(),
    )
    .await?;
    Ok(())
}

pub async fn get_creds_and_config(profile: &str) -> Result<(Credentials, Config), CliError> {
    let creds = get_creds_for_profile(profile).await?;
    let config = get_config_for_profile(profile).await?;

    Ok((creds, config))
}

pub async fn get_creds_for_profile(profile: &str) -> Result<Credentials, CliError> {
    let credentials_file = read_credentials().await?;

    get_session_token(&credentials_file).or_else(|| {
        credentials_file.get(profile, "token")
    }).map(|credentials| {
        Ok(Credentials {
            token: credentials,
        })
    }).unwrap_or_else(|| {
        Err(CliError{
            msg: format!("failed to get credentials for profile {profile}, please run 'momento configure' to configure your profile")
        })
    })
}

async fn read_credentials() -> Result<Ini, CliError> {
    let path = get_credentials_file_path()?;
    read_ini_file(&path).await
}

pub async fn get_config_for_profile(profile: &str) -> Result<Config, CliError> {
    let path = get_config_file_path()?;
    let configs = match read_ini_file(&path).await {
        Ok(c) => c,
        Err(e) => return Err(CliError {
            msg: format!("failed to read credentials, please run 'momento configure' to setup credentials. Root cause: {e:?}")
        }),
    };

    let cache_result = match configs.get(profile, "cache") {
        Some(c) => c,
        None => return Err(CliError{
            msg: format!("failed to get cache config for profile {profile}, please run 'momento configure' to configure your profile")
        }),
    };

    let ttl_result = match configs.get(profile, "ttl") {
        Some(c) => c,
        None => return Err(CliError{
            msg: format!("failed to get ttl config for profile {profile}, please run 'momento configure' to configure your profile")
        }),
    };

    Ok(Config {
        cache: cache_result,
        ttl: ttl_result.parse::<u64>().map_err(|e| CliError {
            msg: format!("could not parse a u64: {e:?}"),
        })?,
    })
}
