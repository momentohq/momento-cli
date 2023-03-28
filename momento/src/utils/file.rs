use std::path::PathBuf;

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
pub static BASE_DIR: &str = ".momento";
pub static SESSION_TOKEN_DIR: &str = "cache";
pub static SESSION_TOKEN_FILE_PATH: &str = "cache/session-tokens";
pub static PROFILE_FILE_NAME: &str = "config";

// Validate files exist, if they don't, make em

pub async fn create_necessary_files() -> Result<(), CliError> {
    match fs::create_dir_all(
        &(get_momento_config_dir()?.join(SESSION_TOKEN_DIR))
            .display()
            .to_string(),
    )
    .await
    {
        Ok(_) => (),
        Err(e) => {
            return Err(CliError {
                msg: format!("failed to create directory: {e}"),
            })
        }
    };

    validate_exist_or_create_ini(&get_config_file_path()?).await?;
    validate_exist_or_create_ini(&get_credentials_file_path()?).await?;

    Ok(())
}

async fn validate_exist_or_create_ini(path: &PathBuf) -> Result<(), CliError> {
    if !path.exists() {
        create_file(path).await?;
    }
    set_file_read_write(&path)
        .await
        .map_err(Into::<CliError>::into)?;
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
            msg: format!("failed to create file {:?}, error: {e}", path),
        }),
    }
}

// Read ini files

pub fn read_profile_ini() -> Result<Ini, CliError> {
    let profile_path = get_config_file_path()?;
    read_ini(&profile_path.display().to_string())
}

pub fn read_session_token_ini() -> Result<Ini, CliError> {
    let creds_path = get_credentials_file_path()?;
    read_ini(&creds_path.display().to_string())
}

fn read_ini(path: &str) -> Result<Ini, CliError> {
    let mut config = Ini::new_cs();
    match config.load(path) {
        Ok(_) => Ok(config),
        Err(e) => Err(CliError {
            msg: format!("failed to read session token file: {e}"),
        }),
    }
}

// Get file paths

fn get_momento_config_dir() -> Result<PathBuf, CliError> {
    let env_var = std::env::var(ENV_VAR_NAME_MOMENTO_CONFIG_DIR);

    if let Ok(val) = env_var {
        return Ok(PathBuf::from(val));
    }
    // If the env var isn't set we default to ~/.momento
    let home = home_dir().ok_or_else(|| CliError {
        msg: "could not find home dir".to_string(),
    })?;
    Ok(PathBuf::from(home).join(BASE_DIR))
}

fn get_credentials_file_path() -> Result<PathBuf, CliError> {
    Ok(get_momento_config_dir()?.join(SESSION_TOKEN_FILE_PATH))
}

fn get_config_file_path() -> Result<PathBuf, CliError> {
    Ok(get_momento_config_dir()?.join(PROFILE_FILE_NAME))
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
    fn update_config_for_profile(&self, profile: &str) -> Result<(), CliError>;
    fn get_credentials_for_profile(&self, profile: &str) -> Result<Credentials, CliError>;
    fn update_credentials_for_profile(&self, profile: &str) -> Result<(), CliError>;

    fn get_ini_value_requried(&self, profile: &str, key: &str) -> Result<String, CliError>;
    fn get_ini_value_uint_required(&self, profile: &str, key: &str) -> Result<u64, CliError>;
    fn get_ini_value_int_required(&self, profile: &str, key: &str) -> Result<i64, CliError>;
}

impl IniHelpers for Ini {
    fn get_config_for_profile(&self, profile: &str) -> Result<Config, CliError> {
        Ok(Config::new(
            self.get_ini_value_requried(profile, CONFIG_CACHE_KEY)?,
            self.get_ini_value_uint_required(profile, CONFIG_TTL_KEY)?,
        ))
    }

    fn update_config_for_profile(&self, profile: &str) -> Result<(), CliError> {
        let file_path = match get_config_file_path() {
            Ok(valid_path) => valid_path,
            Err(_) => return Err(CliError {
                msg: format!("failed to update config for profile {profile}, please run 'momento configure' to configure your profile"),
            }),
        };
        self.write(&file_path).map_err(|e| CliError {
            msg: format!(
                "Failed to update profile ini file, {:?}: {e}",
                file_path.file_name()
            ),
        })?;
        Ok(())
    }

    fn get_credentials_for_profile(&self, profile: &str) -> Result<Credentials, CliError> {
        let token = self.get_ini_value_requried(profile, CREDENTIALS_TOKEN_KEY)?;
        return match self.getint(profile, CREDENTIALS_VALID_FOR_KEY) {
            Ok(Some(valid_for)) => Ok(Credentials::new(token, Some(valid_for))),
            Ok(None) => Ok(Credentials::valid_forever(token)),
            Err(_) => Err(CliError {
                msg: format!("failed to get config for profile {profile}, please run 'momento configure' to configure your profile"),
            }),
        };
    }

    fn update_credentials_for_profile(&self, profile: &str) -> Result<(), CliError> {
        let file_path = match get_credentials_file_path() {
            Ok(valid_path) => valid_path,
            Err(_) => return Err(CliError {
                msg: format!("failed to update config for profile {profile}, please run 'momento configure' to configure your profile"),
            }),
        };
        self.write(&file_path).map_err(|e| CliError {
            msg: format!(
                "Failed to update session-tokens ini file, {:?}: {e}",
                file_path.file_name()
            ),
        })?;
        Ok(())
    }

    fn get_ini_value_requried(&self, profile: &str, key: &str) -> Result<String, CliError> {
        return match self.get(profile, key) {
            Some(value) => Ok(value),
            None => Err(CliError {
                msg: format!("failed to get {key} for profile {profile}, please run 'momento configure' to configure your profile"),
            }),
        };
    }

    fn get_ini_value_uint_required(&self, profile: &str, key: &str) -> Result<u64, CliError> {
        let uint_result = match self.getuint(profile, key) {
            Ok(uint) => uint,
            Err(_) => {
                return Err(CliError {
                    msg: format!("failed to parse {key} for profile {profile}, please run 'momento configure' to configure your profile"),
                })
            }
        };
        return match uint_result {
            Some(value) => Ok(value),
            None => Err(CliError {
                msg: format!("failed to get {key} for profile {profile}, please run 'momento configure' to configure your profile"),
            }),
        };
    }

    fn get_ini_value_int_required(&self, profile: &str, key: &str) -> Result<i64, CliError> {
        let uint_result = match self.getint(profile, key) {
            Ok(uint) => uint,
            Err(_) => {
                return Err(CliError {
                    msg: format!("failed to get {key} for profile {profile}, please run 'momento configure' to configure your profile"),
                })
            }
        };
        return match uint_result {
            Some(value) => Ok(value),
            None => Err(CliError {
                msg: format!("failed to get {key} for profile {profile}, please run 'momento configure' to configure your profile"),
            }),
        };
    }
}
