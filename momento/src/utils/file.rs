use std::path::{Path, PathBuf};

use configparser::ini::Ini;
use home::home_dir;
use log::debug;

use tokio::fs::{self, File};

use crate::{
    config::{
        Config, Credentials, CONFIG_CACHE_KEY, CONFIG_TTL_KEY, CREDENTIALS_TOKEN_KEY,
        CREDENTIALS_VALID_FOR_KEY, ENV_VAR_NAME_MOMENTO_CONFIG_DIR,
    },
    error::CliError,
};

use super::messages::failed_to_get_profile;
pub static CONFIG_DEFAULT_PROFILE_NAME: &str = "default";
pub static LOGIN_DEFAULT_SESSION_PROFILE_NAME: &str = "default";
pub static USER_DEFAULT_SESSION_PROFILE_NAME: &str = "default";

pub static BASE_DIR: &str = ".momento";
pub static SESSION_TOKEN_DIR: &str = "cache";

pub static PROFILE_FILE_NAME: &str = "config";
pub static SESSION_TOKEN_FILE_PATH: &str = "cache/session-tokens";
pub static USER_SESSION_TOKEN_FILE_PATH: &str = "credentials";

// Validate files exist; if they don't, make 'em
pub async fn create_necessary_files() -> Result<(), CliError> {
    fs::create_dir_all(
        &(get_momento_config_dir()?.join(SESSION_TOKEN_DIR))
            .to_str()
            .ok_or_else(|| CliError {
                msg: "Could not encode the momento directory path as a string".to_string(),
            })?,
    )
    .await
    .map_err(|e| CliError {
        msg: format!("failed to create directory: {e}"),
    })?;

    validate_exist_or_create_ini(&get_config_file_path()?).await?;
    validate_exist_or_create_ini(&get_credentials_file_path()?).await?;
    validate_exist_or_create_ini(&get_user_credentials_file_path()?).await?;

    Ok(())
}

async fn validate_exist_or_create_ini(path: &PathBuf) -> Result<(), CliError> {
    if !path.exists() {
        create_file(path).await?;
        set_file_read_write(path)
            .await
            .map_err(Into::<CliError>::into)?;
    }
    Ok(())
}

async fn create_file(path: &PathBuf) -> Result<(), CliError> {
    let res = File::create(&path).await;
    match res {
        Ok(_) => {
            debug!("created file {:?}", path);
            Ok(())
        }
        Err(e) => Err(CliError {
            msg: format!("failed to create file {path:?}, error: {e}"),
        }),
    }
}

// Read ini files

pub async fn read_profile_ini() -> Result<Ini, CliError> {
    let profile_path = get_config_file_path()?;
    read_ini_with_custom_default(&profile_path, Some(CONFIG_DEFAULT_PROFILE_NAME)).await
}

pub async fn read_session_token_ini() -> Result<Ini, CliError> {
    let session_token_path = get_credentials_file_path()?;
    read_ini_with_custom_default(
        &session_token_path,
        Some(LOGIN_DEFAULT_SESSION_PROFILE_NAME),
    )
    .await
}

pub async fn read_user_session_token_ini() -> Result<Ini, CliError> {
    let user_session_token_path = get_user_credentials_file_path()?;
    read_ini_with_custom_default(
        &user_session_token_path,
        Some(USER_DEFAULT_SESSION_PROFILE_NAME),
    )
    .await
}

async fn read_ini_with_custom_default(
    path: &Path,
    default_section_overide: Option<&str>,
) -> Result<Ini, CliError> {
    create_necessary_files().await?;
    let mut config = Ini::new();
    if let Some(new_default) = default_section_overide {
        config.set_default_section(new_default)
    }
    match config.load(path.to_str().ok_or_else(|| CliError {
        msg: "Failed to get ini path".to_string(),
    })?) {
        Ok(_) => Ok(config),
        Err(e) => Err(CliError {
            msg: format!("failed to read file: {e}"),
        }),
    }
}

// Get file paths
fn get_credentials_file_path() -> Result<PathBuf, CliError> {
    Ok(get_momento_config_dir()?.join(SESSION_TOKEN_FILE_PATH))
}

fn get_user_credentials_file_path() -> Result<PathBuf, CliError> {
    Ok(get_momento_config_dir()?.join(USER_SESSION_TOKEN_FILE_PATH))
}

fn get_config_file_path() -> Result<PathBuf, CliError> {
    Ok(get_momento_config_dir()?.join(PROFILE_FILE_NAME))
}

fn get_momento_config_dir() -> Result<PathBuf, CliError> {
    let env_var = std::env::var(ENV_VAR_NAME_MOMENTO_CONFIG_DIR);

    if let Ok(val) = env_var {
        return Ok(PathBuf::from(val));
    }
    // If the env var isn't set we default to ~/.momento
    let home = home_dir().ok_or_else(|| CliError {
        msg: "could not find home dir".to_string(),
    })?;
    Ok(home.join(BASE_DIR))
}

#[cfg(target_os = "linux")]
async fn set_file_read_write(path: &PathBuf) -> Result<(), CliError> {
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
async fn set_file_read_write(path: &PathBuf) -> Result<(), CliError> {
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
async fn set_file_read_write(path: &PathBuf) -> Result<(), CliError> {
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
async fn set_file_read_write(path: &PathBuf) -> Result<(), CliError> {
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

pub trait IniHelpers {
    fn get_config_for_profile(&self, profile: &str) -> Result<Config, CliError>;
    fn get_credentials_for_profile(&self, profile: &str) -> Result<Credentials, CliError>;

    fn write_self_to_the_config_file(&self) -> Result<(), &'static str>;
    fn write_self_to_the_credentials_file(&self) -> Result<(), &'static str>;
    fn write_self_to_the_user_credentials_file(&self) -> Result<(), &'static str>;

    fn get_ini_value_required(&self, profile: &str, key: &str) -> Result<String, CliError>;
    fn get_ini_value_uint_required(&self, profile: &str, key: &str) -> Result<u64, CliError>;
    fn get_ini_value_int_required(&self, profile: &str, key: &str) -> Result<i64, CliError>;
}

impl IniHelpers for Ini {
    fn get_config_for_profile(&self, profile: &str) -> Result<Config, CliError> {
        Ok(Config {
            cache: self.get_ini_value_required(profile, CONFIG_CACHE_KEY)?,
            ttl: self.get_ini_value_uint_required(profile, CONFIG_TTL_KEY)?,
        })
    }

    fn get_credentials_for_profile(&self, profile: &str) -> Result<Credentials, CliError> {
        let token = self.get_ini_value_required(profile, CREDENTIALS_TOKEN_KEY)?;
        match self.getint(profile, CREDENTIALS_VALID_FOR_KEY) {
            Ok(Some(valid_for)) => Ok(Credentials::new(token, Some(valid_for))),
            Ok(None) => Ok(Credentials::valid_forever(token)),
            Err(_) => Err(CliError {
                msg: failed_to_get_profile(profile),
            }),
        }
    }

    fn write_self_to_the_config_file(&self) -> Result<(), &'static str> {
        let file_path = match get_config_file_path() {
            Ok(valid_path) => valid_path,
            Err(e) => {
                log::debug!("get_config_file_path failed: {e:?}");
                return Err("Failed to get config file path");
            }
        };
        self.write(file_path)
            .map_err(|_e| "failed to write to config file")?;
        Ok(())
    }

    fn write_self_to_the_credentials_file(&self) -> Result<(), &'static str> {
        let file_path = match get_credentials_file_path() {
            Ok(valid_path) => valid_path,
            Err(e) => {
                log::debug!("get_credentials_file_path failed: {e:?}");
                return Err("Failed to get user credentials file path");
            }
        };
        self.write(file_path)
            .map_err(|_e| "failed to write to credentials file")?;
        Ok(())
    }

    fn write_self_to_the_user_credentials_file(&self) -> Result<(), &'static str> {
        let file_path = match get_user_credentials_file_path() {
            Ok(valid_path) => valid_path,
            Err(e) => {
                log::debug!("get_credentials_file_path failed: {e:?}");
                return Err("Failed to get credentials file path");
            }
        };
        self.write(file_path)
            .map_err(|_e| "failed to write to user credentials file")?;
        Ok(())
    }

    fn get_ini_value_required(&self, profile: &str, key: &str) -> Result<String, CliError> {
        self.get(profile, key).ok_or_else(|| CliError {
            msg: failed_to_get_profile(profile),
        })
    }

    fn get_ini_value_uint_required(&self, profile: &str, key: &str) -> Result<u64, CliError> {
        Ok(self
            .getuint(profile, key)
            .map_err(|e| {
                log::debug!(
                "Uh oh. We failed to get the uint value from {profile:?} with key {key:?}: {e:?}"
            )
            })
            .map_err(|_| CliError {
                msg: failed_to_get_profile(profile),
            })?
            .unwrap_or_default())
    }

    fn get_ini_value_int_required(&self, profile: &str, key: &str) -> Result<i64, CliError> {
        Ok(self
            .getint(profile, key)
            .map_err(|e| {
                log::debug!(
                "Uh oh. We failed to get the int value from {profile:?} with key {key:?}: {e:?}"
            )
            })
            .map_err(|_| CliError {
                msg: failed_to_get_profile(profile),
            })?
            .unwrap_or_default())
    }
}
