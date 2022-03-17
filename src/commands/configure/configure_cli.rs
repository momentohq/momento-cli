use log::info;
use std::path::Path;
use tokio::fs;

use crate::{
    commands::cache::cache_cli::create_cache,
    config::{Config, Credentials, FileTypes},
    error::CliError,
    utils::{
        file::{
            create_file, get_config_file_path, get_credentials_file_path, get_momento_dir,
            open_file, prompt_user_for_input, read_file_contents, set_file_read_write,
            set_file_readonly, write_to_file,
        },
        ini_config::{
            add_new_profile_to_config, add_new_profile_to_credentials, update_profile_values,
        },
        user::{get_config_for_profile, get_creds_for_profile},
    },
};

pub async fn configure_momento(profile_name: &str) -> Result<(), CliError> {
    let credentials = prompt_user_for_creds(profile_name).await?;
    let config = prompt_user_for_config(profile_name).await?;

    let momento_dir = get_momento_dir();
    let credentials_file_path = get_credentials_file_path();
    let config_file_path = get_config_file_path();

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
    match create_cache(config.cache, credentials.token).await {
        Ok(_) => info!("default cache successfully created"),
        Err(e) => {
            if e.msg.contains("already exists") {
                info!("default cache already exists");
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

async fn prompt_user_for_config(profile_name: &str) -> Result<Config, CliError> {
    let current_config = get_config_for_profile(profile_name)
        .await
        .unwrap_or_default();

    let prompt_cache = if current_config.cache.is_empty() {
        "default-cache"
    } else {
        current_config.cache.as_str()
    };
    let cache_name = match prompt_user_for_input("Default Cache", prompt_cache, false).await {
        Ok(s) => s,
        Err(e) => return Err(e),
    };
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
    let ttl = match prompt_user_for_input(
        "Default Ttl Seconds",
        prompt_ttl.to_string().as_str(),
        false,
    )
    .await?
    .parse::<u32>()
    {
        Ok(ttl) => ttl,
        Err(e) => {
            return Err(CliError {
                msg: format!("failed to parse ttl: {}", e),
            })
        }
    };

    Ok(Config {
        cache: cache_name_to_use,
        ttl,
    })
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
        set_file_read_write(path).await.unwrap();
        match file_types.clone() {
            FileTypes::Credentials(cr) => {
                match add_new_profile_to_credentials(profile_name, path, cr).await {
                    Ok(_) => {}
                    Err(e) => return Err(e),
                }
                // explicitly revoking that access
                set_file_readonly(path).await.unwrap();
            }
            FileTypes::Config(cf) => {
                match add_new_profile_to_config(profile_name, path, cf).await {
                    Ok(_) => {}
                    Err(e) => return Err(e),
                }
            }
        }
    } else {
        // If file already exists, figure out any profiles exist in the file
        set_file_read_write(path).await.unwrap();
        let file = open_file(path).await.unwrap();
        let mut line_array = read_file_contents(file).await;
        // Determine if  file contains profiles
        match find_profile_start(line_array.clone()) {
            // profile_line_num_array contains line number for profile: e.g. [1, 4, 7]
            Some(profile_line_num_array) => {
                // If profile_name does not exist yet, add new profile and token value
                if !does_profile_name_exist(line_array.clone(), profile_name) {
                    match file_types.clone() {
                        FileTypes::Credentials(cr) => {
                            push_to_line_array(
                                &mut line_array,
                                FileTypes::Credentials(cr),
                                profile_name,
                            );
                        }
                        FileTypes::Config(cf) => {
                            push_to_line_array(
                                &mut line_array,
                                FileTypes::Config(cf),
                                profile_name,
                            );
                        }
                    }
                } else {
                    // If profile_name already exists, update token value
                    let line_num_of_existing_profile =
                        find_existing_profile_start(line_array.clone(), profile_name);
                    match file_types.clone() {
                        FileTypes::Credentials(cr) => {
                            update_profile_values(
                                profile_line_num_array,
                                line_num_of_existing_profile,
                                &mut line_array,
                                FileTypes::Credentials(cr),
                            );
                        }
                        FileTypes::Config(cf) => {
                            update_profile_values(
                                profile_line_num_array,
                                line_num_of_existing_profile,
                                &mut line_array,
                                FileTypes::Config(cf),
                            );
                        }
                    }
                }
                match write_to_file(path, line_array.clone()).await {
                    Ok(_) => {}
                    Err(e) => return Err(e),
                }
            }
            None => {
                // If no profile is found, check there is any contents inside of credentials file.
                // If no content was found, write new credentials to the file.
                if line_array.is_empty() {
                    match file_types.clone() {
                        FileTypes::Credentials(cr) => {
                            match add_new_profile_to_credentials(profile_name, path, cr).await {
                                Ok(_) => {}
                                Err(e) => return Err(e),
                            }
                            // explicitly revoking that access
                            set_file_readonly(path).await.unwrap();
                        }
                        FileTypes::Config(cf) => {
                            match add_new_profile_to_config(profile_name, path, cf).await {
                                Ok(_) => {}
                                Err(e) => return Err(e),
                            }
                        }
                    }
                } else {
                    // If there is (such as just comments), then leave it as and new profile and token value
                    match file_types.clone() {
                        FileTypes::Credentials(cr) => {
                            push_to_line_array(
                                &mut line_array,
                                FileTypes::Credentials(cr),
                                profile_name,
                            );
                        }
                        FileTypes::Config(cf) => {
                            push_to_line_array(
                                &mut line_array,
                                FileTypes::Config(cf),
                                profile_name,
                            );
                        }
                    }
                    match write_to_file(path, line_array.clone()).await {
                        Ok(_) => {}
                        Err(e) => return Err(e),
                    }
                }
            }
        }
        match file_types {
            FileTypes::Credentials(_) => {
                set_file_readonly(path).await.unwrap();
            }
            FileTypes::Config(_) => {}
        }
    }
    Ok(())
}

fn find_profile_start(line_array: Vec<String>) -> Option<Vec<usize>> {
    let mut counter = 0;
    let mut profile_counter;
    let line_array_len = line_array.len();
    let mut profile_start_line_num_array: Vec<usize> = vec![];
    while counter < line_array_len {
        let line = line_array[counter].trim();
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

fn does_profile_name_exist(line_array: Vec<String>, profile_name: &str) -> bool {
    for line in line_array.iter() {
        let trimmed_line = line.replace('\n', "");
        if trimmed_line.eq(&format!("[{}]", profile_name)) {
            return true;
        }
    }
    false
}

fn find_existing_profile_start(line_array: Vec<String>, profile_name: &str) -> usize {
    let mut counter = 0;
    let line_array_len = line_array.len();

    while counter < line_array_len {
        let trimmed_line = line_array[counter].replace('\n', "");
        if trimmed_line.eq(&format!("[{}]", profile_name)) {
            return counter;
        }
        counter += 1;
    }
    counter
}

fn push_to_line_array(line_array: &mut Vec<String>, file_types: FileTypes, profile_name: &str) {
    match file_types {
        FileTypes::Credentials(cr) => {
            line_array.push('\n'.to_string());
            line_array.push(format!("[{}]\n", profile_name));
            line_array.push(format!("token={}\n", cr.token));
        }
        FileTypes::Config(cf) => {
            line_array.push('\n'.to_string());
            line_array.push(format!("[{}]\n", profile_name));
            line_array.push(format!("cache={}", cf.cache));
            line_array.push(format!("ttl={}", cf.ttl));
        }
    }
}
