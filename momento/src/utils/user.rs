use configparser::ini::Ini;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};

use log::debug;

use crate::config::{
    Config, Credentials, CONFIG_CACHE_KEY, CONFIG_TTL_KEY, CREDENTIALS_TOKEN_KEY,
    CREDENTIALS_VALID_FOR_KEY,
};

use crate::error::CliError;

use super::file::{
    read_profile_ini, read_session_token_ini, read_user_session_token_ini, IniHelpers,
    LOGIN_DEFAULT_SESSION_PROFILE_NAME, USER_DEFAULT_SESSION_PROFILE_NAME,
};
use super::messages::failed_to_get_profile;

pub async fn get_creds_and_config(profile: &str) -> Result<(Credentials, Config), CliError> {
    let creds = get_credentials_for_profile(profile).await?;
    let config = get_config_for_profile(profile).await?;

    Ok((creds, config))
}

pub async fn update_login_credentials(
    profile: &str,
    session_token: &Credentials,
) -> Result<(), CliError> {
    update_credentials(
        profile,
        false,
        session_token,
        read_user_session_token_ini().await?,
    )
}

pub async fn update_user_credentials(
    profile: &str,
    session_token: &Credentials,
) -> Result<(), CliError> {
    update_credentials(
        profile,
        true,
        session_token,
        read_user_session_token_ini().await?,
    )
}

fn update_credentials(
    profile: &str,
    is_user_managed_creds: bool,
    session_token: &Credentials,
    mut session_ini: Ini,
) -> Result<(), CliError> {
    session_ini.set(
        profile,
        CREDENTIALS_TOKEN_KEY,
        Some(session_token.clone().token),
    );

    if let Some(valid) = &session_token.valid_for {
        session_ini.set(profile, CREDENTIALS_VALID_FOR_KEY, Some(valid.to_string()));
    }
    if is_user_managed_creds {
        session_ini
            .write_self_to_the_user_credentials_file()
            .map_err(|e| CliError {
                msg: format!("Failed to write credentials to credential file: {e:?}"),
            })
    } else {
        session_ini
            .write_self_to_the_credentials_file()
            .map_err(|e| CliError {
                msg: format!("Failed to write credentials to session-token file: {e:?}"),
            })
    }
}

pub async fn update_profile(profile: &str, config: &Config) -> Result<(), CliError> {
    let mut config_ini = read_profile_ini().await?;
    config_ini.set(profile, CONFIG_CACHE_KEY, Some(config.cache.clone()));
    config_ini.set(profile, CONFIG_TTL_KEY, Some(config.ttl.to_string()));
    config_ini
        .write_self_to_the_config_file()
        .map_err(|e| CliError {
            msg: format!("Failed to write profile to config file: {e:?}"),
        })
}

pub async fn get_config_for_profile(profile: &str) -> Result<Config, CliError> {
    let config_ini = read_profile_ini().await?;
    config_ini.get_config_for_profile(profile)
}

pub async fn get_credentials_for_profile(profile: &str) -> Result<Credentials, CliError> {
    // Order of credentials lookup
    // 1. get credentials with user supplied profile
    // 2. get credentials with user default profile
    // 3. get credentials with momento login profile
    if let Ok(creds) = read_user_session_token_ini()
        .await?
        .get_credentials_for_profile(profile)
    {
        return Ok(creds);
    }

    if let Ok(creds) = read_user_session_token_ini()
        .await?
        .get_credentials_for_profile(USER_DEFAULT_SESSION_PROFILE_NAME)
    {
        log::debug!("Failed to find credentials for profile {profile}, falling back to user default credentials");
        return Ok(creds);
    }

    if let Ok(creds) = read_session_token_ini()
        .await?
        .get_credentials_for_profile(LOGIN_DEFAULT_SESSION_PROFILE_NAME)
    {
        log::debug!("Failed to find credentials for profile {profile}, falling back to momento login credentials");
        return Ok(creds);
    }

    Err(CliError {
        msg: failed_to_get_profile(profile),
    })
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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::fs;
    use tempdir::TempDir;
    use uuid::Uuid;

    use crate::{config::Credentials, utils::file::LOGIN_DEFAULT_SESSION_PROFILE_NAME};

    use super::{get_credentials_for_profile, update_login_credentials, update_user_credentials};

    fn initlize_test_setup() {
        let test_momento_config_dir = TempDir::new(&format!("momento-cli-{}", Uuid::new_v4()))
            .expect("Unable to create temp dir");
        let test_momento_config_dir_path = fs::canonicalize(test_momento_config_dir.path())
            .expect("Unable to canonicalize path")
            .into_os_string()
            .into_string()
            .expect("Unable to convert canonical path to string");
        std::env::set_var("MOMENTO_CONFIG_DIR", test_momento_config_dir_path);
    }

    #[tokio::test]
    async fn create_and_use_user_creds_over_login() {
        initlize_test_setup();
        update_user_credentials(
            "default",
            &Credentials {
                token: "user-configured-credentials".to_string(),
                valid_for: None,
            },
        )
        .await
        .expect("Couldn't create user credentials");

        update_login_credentials(
            LOGIN_DEFAULT_SESSION_PROFILE_NAME,
            &Credentials {
                token: "login-configured-credentials".to_string(),
                valid_for: None,
            },
        )
        .await
        .expect("Couldn't create user credentials");

        match get_credentials_for_profile("default").await {
            Ok(creds) => assert_eq!(creds.token, "user-configured-credentials"),
            Err(e) => panic!("Whoa there! We failed to get any credentails, {e:?}",),
        }

        initlize_test_setup();
        update_user_credentials(
            "user",
            &Credentials {
                token: "user-configured-credentials".to_string(),
                valid_for: None,
            },
        )
        .await
        .expect("Couldn't create user credentials");

        update_login_credentials(
            LOGIN_DEFAULT_SESSION_PROFILE_NAME,
            &Credentials {
                token: "login-configured-credentials".to_string(),
                valid_for: None,
            },
        )
        .await
        .expect("Couldn't create user credentials");

        match get_credentials_for_profile("user").await {
            Ok(creds) => assert_eq!(creds.token, "user-configured-credentials"),
            Err(e) => panic!("Whoa there! We failed to get any credentails, {e:?}"),
        }

        match get_credentials_for_profile("invalid-profile-fallback-to-login").await {
            Ok(creds) => assert_eq!(creds.token, "login-configured-credentials"),
            Err(e) => panic!("Whoa there! We failed to get any credentails, {e:?}"),
        }
    }
}
