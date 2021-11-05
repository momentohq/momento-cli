use log::debug;
use std::path::Path;

use maplit::hashmap;
use tokio::{
    fs::{self, File},
    io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader},
};

use crate::{
    credentials::{Credentials, CredentialsConfig},
    utils::{get_credentials_file_path, get_momento_dir, read_toml_file},
};

pub async fn configure_momento() {
    let token = prompt_user_for_input("Token: ").await;
    let momento_dir = get_momento_dir();
    let credentials_file_path = get_credentials_file_path();

    create_dir_if_not_exists(&momento_dir).await;
    create_file_if_not_exists(&credentials_file_path).await;

    let default_credentials = Credentials { token: token };
    add_credentials("default", default_credentials).await;
}

async fn add_credentials(profile: &str, creds: Credentials) {
    let path = get_credentials_file_path();

    let mut credentials_toml = match read_toml_file::<CredentialsConfig>(&path).await {
        Ok(t) => t,
        Err(_) => {
            debug!("credentials file is invalid, most likely we are creating it for the first time. Overwriting it with new profile");
            CredentialsConfig {
                profile: hashmap! {},
            }
        }
    };
    credentials_toml.profile.insert(profile.to_string(), creds);
    let new_creds_string = toml::to_string(&credentials_toml).unwrap();
    write_to_existing_file(&path, &new_creds_string).await;
}

async fn write_to_existing_file(filepath: &str, buffer: &str) {
    match tokio::fs::write(filepath, buffer).await {
        Ok(_) => debug!("wrote buffer to file {}", filepath),
        Err(e) => panic!("failed to write to file {}, error: {}", filepath, e),
    };
}

async fn create_file_if_not_exists(path: &str) {
    if !Path::new(path).exists() {
        let res = File::create(path).await;
        match res {
            Ok(_) => debug!("created file {}", path),
            Err(e) => panic!("failed to create file {}, error: {}", path, e),
        }
    };
}

async fn create_dir_if_not_exists(path: &str) {
    if !Path::new(path).exists() {
        let res = fs::create_dir_all(path).await;
        match res {
            Ok(_) => debug!("created directory {}", path),
            Err(e) => panic!("failed to created directory {}, error: {}", path, e),
        }
    }
}

async fn prompt_user_for_input(prompt: &str) -> String {
    let mut stdout = io::stdout();
    match stdout.write(prompt.as_bytes()).await {
        Ok(_) => debug!("wrote prompt '{}' to stdout", prompt),
        Err(e) => panic!("failed to write prompt to stdout: {}", e),
    };
    match stdout.flush().await {
        Ok(_) => debug!("flushed stdout"),
        Err(e) => panic!("failed to flush stdout: {}", e),
    };
    let stdin = io::stdin();
    let mut buffer = String::new();
    let mut reader = BufReader::new(stdin);
    match reader.read_line(&mut buffer).await {
        Ok(_) => debug!("read line from stdin"),
        Err(e) => panic!("failed to read line from stdin: {}", e),
    };
    return buffer.as_str().trim_end().to_string();
}
