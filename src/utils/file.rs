use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use home::home_dir;
use log::debug;
use serde::de;
use tokio::{fs::{self, File}, io::{self, AsyncWriteExt, BufReader, AsyncBufReadExt}};

use crate::error::CliError;

pub fn get_credentials_file_path() -> String {
    let momento_home = get_momento_dir();
    return format!("{}/credentials.toml", momento_home);
}

pub fn get_config_file_path() -> String {
    let momento_home = get_momento_dir();
    return format!("{}/config.toml", momento_home);
}

pub fn get_momento_dir() -> String {
    let home = home_dir().unwrap();
    return format!("{}/.momento", home.clone().display());
}

pub async fn read_toml_file<T: de::DeserializeOwned>(path: &str) -> Result<T, CliError> {
    let toml_str = match fs::read_to_string(&path).await {
        Ok(s) => s,
        Err(e) => {
            return Err(CliError {
                msg: format!("failed to read file: {}", e),
            })
        }
    };
    match toml::from_str::<T>(&toml_str) {
        Ok(c) => Ok(c),
        Err(e) => Err(CliError {
            msg: format!("failed to parse file: {}", e),
        }),
    }
}

pub async fn write_to_existing_file(filepath: &str, buffer: &str) -> Result<(), CliError> {
    match tokio::fs::write(filepath, buffer).await {
        Ok(_) => debug!("wrote buffer to file {}", filepath),
        Err(e) => {
            return Err(CliError {
                msg: format!("failed to write to file {}, error: {}", filepath, e),
            })
        }
    };
    Ok(())
}

pub async fn create_file_if_not_exists(path: &str) -> Result<(), CliError> {
    if !Path::new(path).exists() {
        let res = File::create(path).await;
        match res {
            Ok(_) => debug!("created file {}", path),
            Err(e) => {
                return Err(CliError {
                    msg: format!("failed to create file {}, error: {}", path, e),
                })
            }
        }
    };
    Ok(())
}

pub async fn set_file_readonly(path: &str) -> Result<(), CliError> {
    let mut perms = match fs::metadata(path).await {
        Ok(p) => p,
        Err(e) => {
            return Err(CliError {
                msg: format!("failed to get file permissions {}", e),
            })
        }
    }
    .permissions();
    perms.set_mode(0o400);
    match fs::set_permissions(path, perms).await {
        Ok(_) => return Ok(()),
        Err(e) => {
            return Err(CliError {
                msg: format!("failed to set file permissions {}", e),
            })
        }
    };
}

pub async fn set_file_read_write(path: &str) -> Result<(), CliError> {
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
        Ok(_) => return Ok(()),
        Err(e) => {
            return Err(CliError {
                msg: format!("failed to set file permissions {}", e),
            })
        }
    };
}

pub async fn prompt_user_for_input(
    prompt: &str,
    default_value: &str,
    is_secret: bool,
) -> Result<String, CliError> {
    let mut stdout = io::stdout();

    let formatted_prompt = if default_value.is_empty() {
        format!("{}: ", prompt)
    } else if is_secret {
        format!("{} [****]: ", prompt)
    } else {
        format!("{} [{}]: ", prompt, default_value)
    };

    match stdout.write(formatted_prompt.as_bytes()).await {
        Ok(_) => debug!("wrote prompt '{}' to stdout", formatted_prompt),
        Err(e) => {
            return Err(CliError {
                msg: format!("failed to write prompt to stdout: {}", e),
            })
        }
    };
    match stdout.flush().await {
        Ok(_) => debug!("flushed stdout"),
        Err(e) => {
            return Err(CliError {
                msg: format!("failed to flush stdout: {}", e),
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
                msg: format!("failed to read line from stdin: {}", e),
            })
        }
    };

    let input = buffer.as_str().trim().to_string();
    if input.is_empty() {
        return Ok(default_value.to_string());
    }
    return Ok(input);
}
