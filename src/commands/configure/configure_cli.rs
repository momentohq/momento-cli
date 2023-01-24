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
            add_new_profile_to_config, add_new_profile_to_credentials, update_profile_values,
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
                msg: format!("failed to create directory: {}", e),
            })
        }
    };
    match add_profile(
        profile_name,
        FileTypes::Credentials(credentials.clone()),
        &credentials_file_path,
    )
    .await
    {
        Ok(_) => {}
        Err(e) => {
            return Err(e);
        }
    }
    match add_profile(
        profile_name,
        FileTypes::Config(config.clone()),
        &config_file_path,
    )
    .await
    {
        Ok(_) => {}
        Err(e) => {
            return Err(e);
        }
    }
    // TODO: Update the endpoint to read from config
    match create_cache(config.cache.clone(), credentials.token, None).await {
        Ok(_) => console_info!(
            "{} successfully created as the default with default TTL of {}s",
            config.cache.clone(),
            config.ttl
        ),
        Err(e) => {
            if e.msg.contains("already exists") {
                console_info!("{} as the default already exists", config.cache);
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
                    msg: format!("failed to parse ttl: {}", e),
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
                msg: format!("failed to get file permissions {}", e),
            })
        }
    }
    .permissions();
    perms.set_mode(0o600);
    match fs::set_permissions(path, perms).await {
        Ok(_) => Ok(()),
        Err(e) => Err(CliError {
            msg: format!("failed to set file permissions {}", e),
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
                msg: format!("failed to get file permissions {}", e),
            })
        }
    }
    .permissions();
    perms.set_mode(0o600);
    match fs::set_permissions(path, perms).await {
        Ok(_) => Ok(()),
        Err(e) => Err(CliError {
            msg: format!("failed to set file permissions {}", e),
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
                msg: format!("failed to get file permissions {}", e),
            })
        }
    }
    .permissions();
    perms.set_mode(0o600);
    match fs::set_permissions(path, perms).await {
        Ok(_) => Ok(()),
        Err(e) => Err(CliError {
            msg: format!("failed to set file permissions {}", e),
        }),
    }
}

#[cfg(target_os = "windows")]
async fn set_file_read_write(path: &str) -> Result<(), CliError> {
    let mut perms = match fs::metadata(path).await {
        Ok(p) => p,
        Err(e) => {
            return Err(CliError {
                msg: format!("failed to get file permissions {}", e),
            })
        }
    }
    .permissions();
    perms.set_readonly(false);
    match fs::set_permissions(path, perms).await {
        Ok(_) => Ok(()),
        Err(e) => Err(CliError {
            msg: format!("failed to set file permissions {}", e),
        }),
    }
}

async fn add_profile(
    profile_name: &str,
    file_types: FileTypes,
    path: &str,
) -> Result<(), CliError> {
    // If file does not exists, create one and set default profile with token
    if !Path::new(path).exists() {
        match create_file(path).await {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        // explicitly allowing read/write access to the file
        set_file_read_write(path).await?;
        match add_new_profile_to_new_file(file_types.clone(), profile_name, path).await {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
    } else {
        // If file already exists, figure out any profiles exist in the file
        set_file_read_write(path).await?;
        let file = open_file(path).await?;
        let file_contents = read_file_contents(file).await?;
        let updated_file_contents: Vec<String>;
        // Determine if  file contains profiles
        match find_profile_start(file_contents.clone()) {
            // existing_profile_line_numbers contains line number for profile: e.g. [1, 4, 7]
            Some(existing_profile_line_numbers) => {
                // If profile_name does not exist yet, add new profile and token value
                if !does_profile_name_exist(file_contents.clone(), profile_name) {
                    updated_file_contents = add_new_profile_to_existing_file(
                        file_types.clone(),
                        file_contents.clone(),
                        profile_name,
                    );
                } else {
                    // If profile_name already exists, update token value
                    let existing_profile_starting_line_num =
                        find_existing_profile_start(file_contents.clone(), profile_name);
                    match file_types.clone() {
                        FileTypes::Credentials(cr) => {
                            updated_file_contents = match update_profile_values(
                                existing_profile_line_numbers,
                                existing_profile_starting_line_num,
                                file_contents.clone(),
                                FileTypes::Credentials(cr),
                            ) {
                                Ok(v) => v,
                                Err(e) => return Err(e),
                            }
                        }
                        FileTypes::Config(cf) => {
                            updated_file_contents = match update_profile_values(
                                existing_profile_line_numbers,
                                existing_profile_starting_line_num,
                                file_contents.clone(),
                                FileTypes::Config(cf),
                            ) {
                                Ok(v) => v,
                                Err(e) => return Err(e),
                            }
                        }
                    }
                }
                match write_to_file(path, updated_file_contents.clone()).await {
                    Ok(_) => {}
                    Err(e) => return Err(e),
                }
            }
            None => {
                // If no profile is found, check there is any contents inside of credentials file.
                // If no content was found, write new credentials to the file.
                if file_contents.is_empty() {
                    match add_new_profile_to_new_file(file_types.clone(), profile_name, path).await
                    {
                        Ok(_) => {}
                        Err(e) => return Err(e),
                    }
                } else {
                    // If there is (such as just comments), then leave it as and new profile and token value
                    updated_file_contents = add_new_profile_to_existing_file(
                        file_types.clone(),
                        file_contents.clone(),
                        profile_name,
                    );
                    match write_to_file(path, updated_file_contents.clone()).await {
                        Ok(_) => {}
                        Err(e) => return Err(e),
                    }
                }
            }
        }
    }
    Ok(())
}

fn find_profile_start(file_contents: Vec<String>) -> Option<Vec<usize>> {
    let mut counter = 0;
    let mut profile_counter;
    let line_array_len = file_contents.len();
    let mut profile_start_line_num_array: Vec<usize> = Vec::new();
    while counter < line_array_len {
        let line = file_contents[counter].trim();
        if line.starts_with('[') && line.ends_with(']') {
            profile_counter = counter;
            // Collect line number of profile
            profile_start_line_num_array.push(profile_counter);
        }
        counter += 1;
    }
    if profile_start_line_num_array.is_empty() {
        None
    } else {
        Some(profile_start_line_num_array)
    }
}

fn does_profile_name_exist(file_contents: Vec<String>, profile_name: &str) -> bool {
    for line in file_contents.iter() {
        let trimmed_line = line.replace('\n', "");
        if trimmed_line.eq(&format!("[{}]", profile_name)) {
            return true;
        }
    }
    false
}

fn find_existing_profile_start(file_contents: Vec<String>, profile_name: &str) -> usize {
    let mut counter = 0;
    let line_array_len = file_contents.len();

    while counter < line_array_len {
        let trimmed_line = file_contents[counter].replace('\n', "");
        if trimmed_line.eq(&format!("[{}]", profile_name)) {
            return counter;
        }
        counter += 1;
    }
    counter
}

fn push_to_file_contents(
    file_contents: Vec<String>,
    file_types: FileTypes,
    profile_name: &str,
) -> Vec<String> {
    let mut updated_file_contents = file_contents;
    match file_types {
        FileTypes::Credentials(cr) => {
            updated_file_contents.push('\n'.to_string());
            updated_file_contents.push(format!("[{}]\n", profile_name));
            updated_file_contents.push(format!("token={}", cr.token));
        }
        FileTypes::Config(cf) => {
            updated_file_contents.push('\n'.to_string());
            updated_file_contents.push(format!("[{}]\n", profile_name));
            updated_file_contents.push(format!("cache={}\n", cf.cache));
            updated_file_contents.push(format!("ttl={}", cf.ttl));
        }
    }
    updated_file_contents
}

async fn add_new_profile_to_new_file(
    file_types: FileTypes,
    profile_name: &str,
    path: &str,
) -> Result<(), CliError> {
    match file_types {
        FileTypes::Credentials(cr) => {
            match add_new_profile_to_credentials(profile_name, path, cr).await {
                Ok(_) => {}
                Err(e) => return Err(e),
            }
            Ok(())
        }
        FileTypes::Config(cf) => match add_new_profile_to_config(profile_name, path, cf).await {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        },
    }
}

fn add_new_profile_to_existing_file(
    file_types: FileTypes,
    file_contents: Vec<String>,
    profile_name: &str,
) -> Vec<String> {
    let updated_file_contents: Vec<String> = match file_types {
        FileTypes::Credentials(cr) => {
            push_to_file_contents(file_contents, FileTypes::Credentials(cr), profile_name)
        }
        FileTypes::Config(cf) => {
            push_to_file_contents(file_contents, FileTypes::Config(cf), profile_name)
        }
    };
    updated_file_contents
}
