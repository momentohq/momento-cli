use home::home_dir;
use serde::de;
use tokio::fs;

use crate::credentials::{Credentials, CredentialsConfig};

pub async fn get_creds_for_profile(profile: Option<String>) -> Credentials {
    let path = get_credentials_file_path();
    let credentials_toml = match read_toml_file::<CredentialsConfig>(&path).await {
        Ok(c) => c,
        Err(_) => panic!(
            "failed to read credentials, please run 'momento configure' to setup credentials"
        ),
    };

    let actual_profile = profile.unwrap_or("default".to_string());

    let creds_result = match credentials_toml.profile.get(&actual_profile.clone()) {
        Some(c) => c,
        None => panic!("failed to get credentials for profile {}, please run 'momento configure' to confiure your profile", &actual_profile.clone()),
    };

    return creds_result.clone();
}

pub fn get_credentials_file_path() -> String {
    let home = home_dir().unwrap();
    return format!("{}/.momento/credentials.toml", home.clone().display());
}

pub fn get_momento_dir() -> String {
    let home = home_dir().unwrap();
    return format!("{}/.momento", home.clone().display());
}

pub async fn read_toml_file<T: de::DeserializeOwned>(path: &str) -> Result<T, &str> {
    let toml_str = match fs::read_to_string(&path).await {
        Ok(s) => s,
        Err(_) => panic!(
            "faile to read file {}, please run 'momento configure'",
            path
        ),
    };
    match toml::from_str::<T>(&toml_str) {
        Ok(c) => Ok(c),
        Err(e) => Err("failed to deserialize toml file"),
    }
}
