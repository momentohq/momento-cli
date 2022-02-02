use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use home::home_dir;
use log::debug;
use serde::de;
use tokio::fs::{self, File};

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
