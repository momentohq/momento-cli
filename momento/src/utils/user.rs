use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};

use log::debug;

use crate::config::{
    Config, Credentials, CONFIG_CACHE_KEY, CONFIG_TTL_KEY, CREDENTIALS_TOKEN_KEY,
    CREDENTIALS_VALID_FOR_KEY,
};

use crate::error::CliError;

use super::file::{read_profile_ini, read_session_token_ini, IniHelpers};

pub async fn get_creds_and_config(profile: &str) -> Result<(Credentials, Config), CliError> {
    let creds = get_credentials_for_profile(profile).await?;
    let config = get_config_for_profile(profile).await?;

    Ok((creds, config))
}

pub fn update_credentials(profile: &str, session_token: &Credentials) -> Result<(), CliError> {
    let mut session_token_ini = read_session_token_ini()?;
    session_token_ini.set(
        profile,
        CREDENTIALS_TOKEN_KEY,
        Some(session_token.clone().token),
    );

    match &session_token.valid_for {
        Some(valid) => {
            session_token_ini.set(profile, CREDENTIALS_VALID_FOR_KEY, Some(valid.to_string()));
        }
        None => (),
    }
    session_token_ini.update_credentials_for_profile(profile)
}

pub fn update_profile(profile: &str, config: &Config) -> Result<(), CliError> {
    let mut config_ini = read_profile_ini()?;
    config_ini.set(profile, CONFIG_CACHE_KEY, Some(config.cache.clone()));
    config_ini.set(profile, CONFIG_TTL_KEY, Some(config.ttl.to_string()));
    config_ini.update_config_for_profile(profile)
}

pub async fn get_config_for_profile(profile: &str) -> Result<Config, CliError> {
    let config_ini = read_profile_ini()?;
    config_ini.get_config_for_profile(profile)
}

pub async fn get_credentials_for_profile(profile: &str) -> Result<Credentials, CliError> {
    let session_tokens_ini = read_session_token_ini()?;
    session_tokens_ini.get_credentials_for_profile(profile)
}

pub async fn prompt_user_for_input(
    prompt: &str,
    default_value: &str,
    is_secret: bool,
) -> Result<String, CliError> {
    let mut stdout = io::stdout();

    let formatted_prompt = if default_value.is_empty() {
        format!("{prompt}: ")
    } else if is_secret {
        format!("{prompt} [****]: ")
    } else {
        format!("{prompt} [{default_value}]: ")
    };

    match stdout.write(formatted_prompt.as_bytes()).await {
        Ok(_) => debug!("wrote prompt '{}' to stdout", formatted_prompt),
        Err(e) => {
            return Err(CliError {
                msg: format!("failed to write prompt to stdout: {e}"),
            })
        }
    };
    match stdout.flush().await {
        Ok(_) => debug!("flushed stdout"),
        Err(e) => {
            return Err(CliError {
                msg: format!("failed to flush stdout: {e}"),
            })
        }
    };
    let stdin = io::stdin();
    let mut buffer = String::new();
    let mut reader = BufReader::new(stdin);
    match reader.read_line(&mut buffer).await {
        Ok(_) => debug!("read line from stdin"),
        Err(e) => {
            return Err(CliError {
                msg: format!("failed to read line from stdin: {e}"),
            })
        }
    };

    let input = buffer.as_str().trim().to_string();
    if input.is_empty() {
        return Ok(default_value.to_string());
    }
    Ok(input)
}
