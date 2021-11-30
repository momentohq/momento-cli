use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use home::home_dir;
use log::debug;
use serde::de;
use tokio::fs::{self, File};

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

pub async fn read_toml_file<T: de::DeserializeOwned>(path: &str) -> Result<T, String> {
    let toml_str = match fs::read_to_string(&path).await {
        Ok(s) => s,
        Err(e) => return Err(format!("failed to read toml file: {}", e)),
    };
    match toml::from_str::<T>(&toml_str) {
        Ok(c) => Ok(c),
        Err(e) => Err(format!("failed to parse toml file: {}", e)),
    }
}

pub async fn write_to_existing_file(filepath: &str, buffer: &str) {
    match tokio::fs::write(filepath, buffer).await {
        Ok(_) => debug!("wrote buffer to file {}", filepath),
        Err(e) => panic!("failed to write to file {}, error: {}", filepath, e),
    };
}

pub async fn create_file_if_not_exists(path: &str) {
    if !Path::new(path).exists() {
        let res = File::create(path).await;
        match res {
            Ok(_) => debug!("created file {}", path),
            Err(e) => panic!("failed to create file {}, error: {}", path, e),
        }
    };
}

pub async fn set_file_readonly(path: &str) -> std::io::Result<()> {
    let mut perms = match fs::metadata(path).await {
        Ok(p) => p,
        Err(e) => panic!("failed to get file permissions {}", e),
    }
    .permissions();
    perms.set_mode(0o400);
    fs::set_permissions(path, perms).await?;
    Ok(())
}

pub async fn set_file_read_write(path: &str) -> std::io::Result<()> {
    let mut perms = match fs::metadata(path).await {
        Ok(p) => p,
        Err(e) => panic!("failed to get file permissions {}", e),
    }
    .permissions();
    perms.set_mode(0o600);
    fs::set_permissions(path, perms).await?;
    Ok(())
}
