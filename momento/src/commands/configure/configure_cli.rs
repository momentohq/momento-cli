use log::warn;
use std::path::Path;
use tokio::fs;

use crate::config::DEFAULT_CACHE_NAME;
use crate::utils::ini_config::{update_config_profile, update_credentials_profile};
use crate::{
    commands::cache::cache_cli::create_cache,
    config::{Config, Credentials},
    error::CliError,
    utils::{
        console::console_info,
        file::{
            create_file, get_config_file_path, get_credentials_file_path, get_momento_config_dir,
            open_file, prompt_user_for_input, read_file_contents, write_to_file,
        },
        ini_config::{
            create_new_config_profile, create_new_credentials_profile, does_profile_name_exist,
        },
        user::{get_config_for_profile, get_creds_for_profile},
    },
};

pub async fn configure_momento(
    quick: bool,
    profile_name: &str,
    api_key_and_endpoint: bool,
    disposable_token: bool,
) -> Result<(), CliError> {
    let credentials =
        prompt_user_for_creds(profile_name, api_key_and_endpoint, disposable_token).await?;
    let config = prompt_user_for_config(quick, profile_name).await?;

    let momento_dir = get_momento_config_dir()?;
    let credentials_file_path = get_credentials_file_path()?;
    let config_file_path = get_config_file_path()?;

    match fs::create_dir_all(momento_dir).await {
        Ok(_) => (),
        Err(e) => {
            return Err(CliError {
                msg: format!("failed to create directory: {e}"),
            })
        }
    };
    let creds_file_contents = ensure_file_exists_and_get_contents(&credentials_file_path).await?;
    let new_creds_file_contents =
        add_or_update_credentials_profile(profile_name, credentials.clone(), creds_file_contents)?;
    write_to_file(
        &credentials_file_path,
        lines_to_file_content(new_creds_file_contents),
    )
    .await?;
    let config_file_contents = ensure_file_exists_and_get_contents(&config_file_path).await?;
    let new_config_file_contents =
        add_or_update_profile_config(profile_name, config.clone(), config_file_contents)?;
    write_to_file(
        &config_file_path,
        lines_to_file_content(new_config_file_contents),
    )
    .await?;

    // TODO: Update the endpoint to read from config
    match create_cache(config.cache.clone(), credentials, None).await {
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

async fn prompt_user_for_creds(
    profile_name: &str,
    api_key_and_endpoint: bool,
    disposable_token: bool,
) -> Result<Credentials, CliError> {
    if api_key_and_endpoint && disposable_token {
        warn!(
            "Both --api-key-and-endpoint and --disposable-token were provided. Proceeding with --api-key-and-endpoint."
        );
    }
    if api_key_and_endpoint {
        return prompt_user_for_api_key_v2().await;
    }
    if disposable_token {
        return prompt_user_for_disposable_token().await;
    }

    if let Ok(existing_creds) = get_creds_for_profile(profile_name).await {
        console_info!(
            "Found existing credentials for profile '{}', do you wish to use those values? (y/n)",
            profile_name
        );
        let use_existing = prompt_user_for_input("Use existing credentials", "y", false).await?;
        if use_existing.to_lowercase() == "y" || use_existing.to_lowercase() == "yes" {
            return Ok(existing_creds);
        }
    }

    console_info!("\n");
    let v2_or_disposable_token = prompt_user_for_input(
        "Are you setting an [1] API key or [2] disposable auth token?",
        "1",
        false,
    )
    .await?;
    if v2_or_disposable_token.trim() == "2" {
        prompt_user_for_disposable_token().await
    } else {
        prompt_user_for_api_key_v2().await
    }
}

async fn prompt_user_for_api_key_v2() -> Result<Credentials, CliError> {
    console_info!("\n");
    console_info!("Please paste your Momento API Key v2 and endpoint.");
    console_info!("  - If you do not have an API Key, use the Momento Console (https://console.gomomento.com) to generate one.");
    console_info!("  - If you do not already know what endpoint to connect to, please refer to https://docs.momentohq.com/platform/regions#resp-and-sdk-endpoints to select an appropriate endpoint.");
    console_info!(
        "  - Windows users: if CTRL-V does not work, try right-click or SHIFT-INSERT to paste."
    );
    console_info!("");

    let api_key_v2 = prompt_user_for_input("API Key", "", true).await?;
    let endpoint = prompt_user_for_input("Endpoint", "", false).await?;

    Ok(Credentials::ApiKeyV2(api_key_v2, endpoint))
}

async fn prompt_user_for_disposable_token() -> Result<Credentials, CliError> {
    console_info!("\n");
    console_info!("Please paste your Momento disposable auth token.");
    console_info!("  - If you do not have a token, use a Momento SDK that supports GenerateDisposableToken to create one (https://docs.momentohq.com/topics/api-reference/auth#generatedisposabletoken).");
    console_info!(
        "  - Windows users: if CTRL-V does not work, try right-click or SHIFT-INSERT to paste."
    );
    console_info!("");

    let token = prompt_user_for_input("Disposable Auth Token", "", true).await?;

    Ok(Credentials::DisposableToken(token))
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

#[cfg(target_os = "linux")]
async fn set_file_read_write(path: &str) -> Result<(), CliError> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = match fs::metadata(path).await {
        Ok(p) => p,
        Err(e) => {
            return Err(CliError {
                msg: format!("failed to get file permissions {e}"),
            })
        }
    }
    .permissions();
    perms.set_mode(0o600);
    match fs::set_permissions(path, perms).await {
        Ok(_) => Ok(()),
        Err(e) => Err(CliError {
            msg: format!("failed to set file permissions {e}"),
        }),
    }
}

#[cfg(target_os = "macos")]
async fn set_file_read_write(path: &str) -> Result<(), CliError> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = match fs::metadata(path).await {
        Ok(p) => p,
        Err(e) => {
            return Err(CliError {
                msg: format!("failed to get file permissions {e}"),
            })
        }
    }
    .permissions();
    perms.set_mode(0o600);
    match fs::set_permissions(path, perms).await {
        Ok(_) => Ok(()),
        Err(e) => Err(CliError {
            msg: format!("failed to set file permissions {e}"),
        }),
    }
}

#[cfg(target_os = "windows")]
async fn set_file_read_write(path: &str) -> Result<(), CliError> {
    let mut perms = match fs::metadata(path).await {
        Ok(p) => p,
        Err(e) => {
            return Err(CliError {
                msg: format!("failed to get file permissions {e}"),
            })
        }
    }
    .permissions();
    perms.set_readonly(false);
    match fs::set_permissions(path, perms).await {
        Ok(_) => Ok(()),
        Err(e) => Err(CliError {
            msg: format!("failed to set file permissions {e}"),
        }),
    }
}

async fn ensure_file_exists_and_get_contents(path: &str) -> Result<Vec<String>, CliError> {
    if !Path::new(path).exists() {
        match create_file(path).await {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
    }
    // explicitly allowing read/write access to the file
    set_file_read_write(path).await?;

    let file = open_file(path).await?;
    read_file_contents(file).await
}

fn lines_to_file_content(lines: Vec<String>) -> String {
    // ensure a single trailing newline
    format!("{}\n", lines.join("\n").trim_end())
}

fn trim_file_contents(lines: Vec<String>) -> Vec<String> {
    // This is dumb and inefficient but we can optimize it later if necessary
    let content = lines.join("\n");
    let trimmed = content.trim();
    if trimmed.is_empty() {
        vec![]
    } else {
        trimmed.split('\n').map(|line| line.to_string()).collect()
    }
}

fn add_or_update_credentials_profile(
    profile_name: &str,
    credentials: Credentials,
    file_contents: Vec<String>,
) -> Result<Vec<String>, CliError> {
    let trimmed_file_contents = trim_file_contents(file_contents);
    // If profile_name does not exist yet, add new profile and token value
    if !does_profile_name_exist(&trimmed_file_contents, profile_name) {
        Ok(add_new_credentials_profile(
            credentials,
            profile_name,
            trimmed_file_contents,
        ))
    } else {
        // If profile_name already exists, update token value
        update_credentials_profile(profile_name, &trimmed_file_contents, credentials)
    }
}

fn add_or_update_profile_config(
    profile_name: &str,
    config: Config,
    file_contents: Vec<String>,
) -> Result<Vec<String>, CliError> {
    let trimmed_file_contents = trim_file_contents(file_contents);
    // If profile_name does not exist yet, add new profile and token value
    if !does_profile_name_exist(&trimmed_file_contents, profile_name) {
        Ok(add_new_config_profile(
            config,
            profile_name,
            trimmed_file_contents,
        ))
    } else {
        // If profile_name already exists, update token value
        update_config_profile(profile_name, &trimmed_file_contents, config)
    }
}

fn add_new_credentials_profile(
    credentials: Credentials,
    profile_name: &str,
    current_file_content: Vec<String>,
) -> Vec<String> {
    let new_profile = create_new_credentials_profile(profile_name, credentials);
    if current_file_content.is_empty() {
        new_profile
    } else {
        [current_file_content, vec!["\n".to_string()], new_profile].concat()
    }
}

fn add_new_config_profile(
    config: Config,
    profile_name: &str,
    current_file_content: Vec<String>,
) -> Vec<String> {
    let new_profile = create_new_config_profile(profile_name, config);
    if current_file_content.is_empty() {
        new_profile
    } else {
        [current_file_content, vec!["\n".to_string()], new_profile].concat()
    }
}

#[cfg(test)]
mod tests {
    use crate::commands::configure::configure_cli::add_or_update_credentials_profile;
    use crate::config::Credentials;

    fn test_file_content(untrimmed_file_contents: &str) -> String {
        format!("{}\n", untrimmed_file_contents.trim())
    }

    fn test_content_to_lines(file_contents: &str) -> Vec<String> {
        file_contents
            .trim()
            .split('\n')
            .map(|line| line.to_string())
            .collect()
    }

    #[test]
    fn add_or_update_credentials_profile_no_existing_file_disposable_token() {
        let existing_content = vec![];
        let updated = add_or_update_credentials_profile(
            "default",
            Credentials::DisposableToken("awesome-token".to_string()),
            existing_content,
        )
        .expect("d'oh");
        let expected = test_file_content(
            "
[default]
token=awesome-token
        ",
        );
        assert_eq!(expected.trim_end(), updated.join("\n"));
    }

    #[test]
    fn add_or_update_credentials_profile_no_existing_file_v2_api_key() {
        let existing_content = vec![];
        let updated = add_or_update_credentials_profile(
            "default",
            Credentials::ApiKeyV2(
                "awesome-api-key".to_string(),
                "awesome-endpoint".to_string(),
            ),
            existing_content,
        )
        .expect("d'oh");
        let expected = test_file_content(
            "
[default]
api_key_v2=awesome-api-key
endpoint=awesome-endpoint
        ",
        );
        assert_eq!(expected.trim_end(), updated.join("\n"));
    }

    #[test]
    fn add_or_update_credentials_profile_empty_existing_file() {
        let existing_content = test_content_to_lines("");
        let updated = add_or_update_credentials_profile(
            "default",
            Credentials::DisposableToken("awesome-token".to_string()),
            existing_content,
        )
        .expect("d'oh");
        let expected = test_file_content(
            "
[default]
token=awesome-token
        ",
        );
        assert_eq!(expected.trim_end(), updated.join("\n"));
    }

    #[test]
    fn add_or_update_credentials_profile_existing_file_existing_profile_same_token() {
        let existing_content = test_content_to_lines(
            "
[default]
token=old-token
        ",
        );
        let updated1 = format!(
            "{}\n",
            add_or_update_credentials_profile(
                "default",
                Credentials::DisposableToken("old-token".to_string()),
                existing_content
            )
            .expect("d'oh")
            .join("\n")
        );
        let updated2 = add_or_update_credentials_profile(
            "default",
            Credentials::DisposableToken("old-token".to_string()),
            test_content_to_lines(&updated1),
        )
        .expect("d'oh");
        let expected = test_file_content(
            "
[default]
token=old-token
        ",
        );
        assert_eq!(expected.trim_end(), updated2.join("\n"));
    }

    #[test]
    fn add_or_update_credentials_profile_existing_file_existing_profile_new_token() {
        let existing_content = test_content_to_lines(
            "
[default]
token=old-token
        ",
        );
        let updated = add_or_update_credentials_profile(
            "default",
            Credentials::DisposableToken("awesome-token".to_string()),
            existing_content,
        )
        .expect("d'oh");
        let expected = test_file_content(
            "
[default]
token=awesome-token
        ",
        );
        assert_eq!(expected.trim_end(), updated.join("\n"));
    }

    #[test]
    fn add_or_update_credentials_profile_existing_token_is_empty() {
        let existing_content = test_content_to_lines(
            "
[default]
token=
        ",
        );
        let updated = add_or_update_credentials_profile(
            "default",
            Credentials::DisposableToken("awesome-token".to_string()),
            existing_content,
        )
        .expect("d'oh");
        let expected = test_file_content(
            "
[default]
token=awesome-token
        ",
        );
        assert_eq!(expected.trim_end(), updated.join("\n"));
    }
}
