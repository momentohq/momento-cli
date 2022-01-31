use log::debug;
use serde::{de::DeserializeOwned, Serialize};

use tokio::{
    fs,
    io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader},
};

use crate::{
    config::{Config, Credentials, Profiles},
    utils::{
        file::{
            create_file_if_not_exists, get_config_file_path, get_credentials_file_path,
            get_momento_dir, read_toml_file, set_file_read_write, set_file_readonly,
            write_to_existing_file,
        },
        user::{get_config_for_profile, get_creds_for_profile},
    },
};

pub async fn configure_momento(profile_name: &str) {
    let credentials = prompt_user_for_creds(profile_name).await;
    let config = prompt_user_for_config(profile_name).await;

    let momento_dir = get_momento_dir();
    let credentials_file_path = get_credentials_file_path();
    let config_file_path = get_config_file_path();

    fs::create_dir_all(momento_dir).await.unwrap();
    create_file_if_not_exists(&credentials_file_path).await;
    create_file_if_not_exists(&config_file_path).await;

    // explicitly allowing read/write access to the credentials file
    set_file_read_write(&credentials_file_path).await.unwrap();
    add_profile(profile_name, credentials, &credentials_file_path).await;
    // explicitly revoking that access
    set_file_readonly(&credentials_file_path).await.unwrap();

    add_profile(profile_name, config, &config_file_path).await;
}

async fn prompt_user_for_creds(profile_name: &str) -> Credentials {
    let current_credentials = get_creds_for_profile(profile_name)
        .await
        .unwrap_or_default();

    let token = prompt_user_for_input("Token", current_credentials.token.as_str(), true).await;

    return Credentials { token };
}

async fn prompt_user_for_config(profile_name: &str) -> Config {
    let current_config = get_config_for_profile(profile_name)
        .await
        .unwrap_or_default();

    let cache_name =
        prompt_user_for_input("Default Cache", current_config.cache.as_str(), false).await;
    let prompt_ttl = if current_config.ttl == 0 { 600 } else { current_config.ttl };
    let ttl = prompt_user_for_input(
        "Default Ttl Seconds",
        prompt_ttl.to_string().as_str(),
        false,
    )
    .await
    .parse::<u32>()
    .unwrap();

    return Config {
        cache: cache_name,
        ttl,
    };
}

async fn add_profile<T>(profile_name: &str, config: T, config_file_path: &str)
where
    T: DeserializeOwned + Default + Serialize,
{
    let mut toml = match read_toml_file::<Profiles<T>>(config_file_path).await {
        Ok(t) => t,
        Err(_) => {
            debug!("config file is invalid, most likely we are creating it for the first time. Overwriting it with new profile");
            Profiles::<T>::default()
        }
    };
    toml.profile.insert(profile_name.to_string(), config);
    let new_profile_string = toml::to_string(&toml).unwrap();
    write_to_existing_file(config_file_path, &new_profile_string).await;
}

async fn prompt_user_for_input(prompt: &str, default_value: &str, is_secret: bool) -> String {
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

    let input = buffer.as_str().trim_end().to_string();
    if input.is_empty() {
        return default_value.to_string();
    }
    return input;
}
