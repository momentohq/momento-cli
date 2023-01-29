use std::path::Path;
use tokio::fs;

use crate::{
    commands::cache::cache_cli::create_cache,
    config::{Config, Credentials, FileTypes},
    error::CliError,
    utils::{
        console::console_info,
        file::{
            create_file, get_config_file_path, get_credentials_file_path, get_momento_dir,
            open_file, prompt_user_for_input, read_file_contents, write_to_file,
        },
        ini_config::{
            create_new_config_profile, create_new_credentials_profile, does_profile_name_exist,
            update_profile_values,
        },
        user::{get_config_for_profile, get_creds_for_profile},
    },
};

pub async fn configure_momento(quick: bool, profile_name: &str) -> Result<(), CliError> {
    let credentials = prompt_user_for_creds(profile_name).await?;
    let config = prompt_user_for_config(quick, profile_name).await?;

    let momento_dir = get_momento_dir()?;
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
    let new_creds_file_contents = add_or_update_profile(
        profile_name,
        FileTypes::Credentials(credentials.clone()),
        creds_file_contents,
    )?;
    write_to_file(
        &credentials_file_path,
        lines_to_file_content(new_creds_file_contents),
    )
    .await?;
    let config_file_contents = ensure_file_exists_and_get_contents(&config_file_path).await?;
    let new_config_file_contents = add_or_update_profile(
        profile_name,
        FileTypes::Config(config.clone()),
        config_file_contents,
    )?;
    write_to_file(
        &config_file_path,
        lines_to_file_content(new_config_file_contents),
    )
    .await?;

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
    let current_credentials = get_creds_for_profile(profile_name)
        .await
        .unwrap_or_default();

    console_info!("Please paste your Momento auth token.  (If you do not have an auth token, use `momento account` to generate one.)");
    console_info!(
        "Windows users: if CTRL-V does not work, try right-click or SHIFT-INSERT to paste."
    );
    console_info!("");

    let token = prompt_user_for_input("Token", current_credentials.token.as_str(), true).await?;

    Ok(Credentials { token })
}

async fn prompt_user_for_config(quick: bool, profile_name: &str) -> Result<Config, CliError> {
    let current_config = get_config_for_profile(profile_name)
        .await
        .unwrap_or_default();

    let prompt_cache = if current_config.cache.is_empty() {
        "default-cache"
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
        "default-cache".to_string()
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

#[cfg(target_os = "ubuntu")]
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

fn add_or_update_profile(
    profile_name: &str,
    file_types: FileTypes,
    file_contents2: Vec<String>,
    // path: &str,
) -> Result<Vec<String>, CliError> {
    let trimmed_file_contents = trim_file_contents(file_contents2);
    // If profile_name does not exist yet, add new profile and token value
    if !does_profile_name_exist(trimmed_file_contents.clone(), profile_name) {
        Ok(add_new_profile(
            file_types.clone(),
            profile_name,
            trimmed_file_contents,
        ))
    } else {
        // If profile_name already exists, update token value
        match file_types {
            FileTypes::Credentials(cr) => {
                match update_profile_values(
                    profile_name,
                    trimmed_file_contents,
                    FileTypes::Credentials(cr),
                ) {
                    Ok(v) => Ok(v),
                    Err(e) => Err(e),
                }
            }
            FileTypes::Config(cf) => {
                match update_profile_values(
                    profile_name,
                    trimmed_file_contents,
                    FileTypes::Config(cf),
                ) {
                    Ok(v) => Ok(v),
                    Err(e) => Err(e),
                }
            }
        }
    }
}

fn add_new_profile(
    file_types: FileTypes,
    profile_name: &str,
    current_file_content: Vec<String>,
) -> Vec<String> {
    let new_profile = match file_types {
        FileTypes::Credentials(cr) => create_new_credentials_profile(profile_name, cr),
        FileTypes::Config(cf) => create_new_config_profile(profile_name, cf),
    };
    if current_file_content.is_empty() {
        new_profile
    } else {
        [current_file_content, vec!["\n".to_string()], new_profile].concat()
    }
}

#[cfg(test)]
mod tests {
    use crate::commands::configure::configure_cli::add_or_update_profile;
    use crate::config::{Credentials, FileTypes};

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
    fn add_or_update_profile_creds_no_existing_file() {
        let existing_content = vec![];
        let updated = add_or_update_profile(
            "default",
            FileTypes::Credentials(Credentials {
                token: "awesome-token".to_string(),
            }),
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
    fn add_or_update_profile_creds_empty_existing_file() {
        let existing_content = test_content_to_lines("");
        let updated = add_or_update_profile(
            "default",
            FileTypes::Credentials(Credentials {
                token: "awesome-token".to_string(),
            }),
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
    fn add_or_update_profile_creds_existing_file_existing_profile_same_token() {
        let existing_content = test_content_to_lines(
            "
[default]
token=old-token
        ",
        );
        let updated1 = format!(
            "{}\n",
            add_or_update_profile(
                "default",
                FileTypes::Credentials(Credentials {
                    token: "old-token".to_string()
                }),
                existing_content
            )
            .expect("d'oh")
            .join("\n")
        );
        let updated2 = add_or_update_profile(
            "default",
            FileTypes::Credentials(Credentials {
                token: "old-token".to_string(),
            }),
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
    fn add_or_update_profile_creds_existing_file_existing_profile_new_token() {
        let existing_content = test_content_to_lines(
            "
[default]
token=old-token
        ",
        );
        let updated = add_or_update_profile(
            "default",
            FileTypes::Credentials(Credentials {
                token: "awesome-token".to_string(),
            }),
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
