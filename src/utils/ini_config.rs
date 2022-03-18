use configparser::ini::Ini;
use regex::Regex;

use crate::{
    config::{Config, Credentials, FileTypes},
    error::CliError,
    utils::file::ini_write_to_file,
};

pub async fn add_new_profile_to_credentials(
    profile_name: &str,
    credentials_file_path: &str,
    credentials: Credentials,
) -> Result<(), CliError> {
    let mut ini_map = Ini::new_cs();
    // Empty default_section for Ini instance so that "default" will be used as a section
    ini_map.set_default_section("");
    ini_map.set(profile_name, "token", Some(credentials.token));
    match ini_write_to_file(ini_map, credentials_file_path).await {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}

pub async fn add_new_profile_to_config(
    profile_name: &str,
    config_file_path: &str,
    config: Config,
) -> Result<(), CliError> {
    let mut ini_map = Ini::new_cs();
    // Empty default_section for Ini instance so that "default" will be used as a section
    ini_map.set_default_section("");
    ini_map.set(profile_name, "cache", Some(config.cache));
    ini_map.set(profile_name, "ttl", Some(config.ttl.to_string()));
    match ini_write_to_file(ini_map, config_file_path).await {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}

pub fn update_profile_values(
    existing_profile_line_numbers: Vec<usize>,
    existing_profile_starting_line_num: usize,
    file_contents: Vec<String>,
    file_types: FileTypes,
) -> Result<Vec<String>, CliError> {
    let num_of_profiles = existing_profile_line_numbers.len();
    let file_contents_len = file_contents.len();
    let mut updated_file_contents: Vec<String> = Vec::new();
    for (counter, line_num) in existing_profile_line_numbers.iter().enumerate() {
        if existing_profile_starting_line_num == *line_num {
            // Case where profile_name is the only or last item in existing_profile_line_numbers
            if counter == num_of_profiles - 1 {
                for n in *line_num..file_contents_len {
                    match file_types {
                        FileTypes::Credentials(ref cr) => {
                            if n == *line_num {
                                updated_file_contents = match replace_value(
                                    file_contents.clone(),
                                    n,
                                    FileTypes::Credentials(cr.clone()),
                                ) {
                                    Ok(v) => v,
                                    Err(e) => return Err(e),
                                }
                            } else {
                                updated_file_contents = match replace_value(
                                    updated_file_contents.clone(),
                                    n,
                                    FileTypes::Credentials(cr.clone()),
                                ) {
                                    Ok(v) => v,
                                    Err(e) => return Err(e),
                                }
                            }
                        }
                        FileTypes::Config(ref cf) => {
                            if n == *line_num {
                                updated_file_contents = match replace_value(
                                    file_contents.clone(),
                                    n,
                                    FileTypes::Config(cf.clone()),
                                ) {
                                    Ok(v) => v,
                                    Err(e) => return Err(e),
                                }
                            } else {
                                updated_file_contents = match replace_value(
                                    updated_file_contents.clone(),
                                    n,
                                    FileTypes::Config(cf.clone()),
                                ) {
                                    Ok(v) => v,
                                    Err(e) => return Err(e),
                                }
                            }
                        }
                    }
                }
            } else {
                // Case where profile_name is at the beginning or at the middle of existing_profile_line_numbers
                for n in existing_profile_line_numbers[counter]
                    ..existing_profile_line_numbers[counter + 1]
                {
                    match file_types {
                        FileTypes::Credentials(ref cr) => {
                            if n == existing_profile_line_numbers[counter] {
                                updated_file_contents = match replace_value(
                                    file_contents.clone(),
                                    n,
                                    FileTypes::Credentials(cr.clone()),
                                ) {
                                    Ok(v) => v,
                                    Err(e) => return Err(e),
                                }
                            } else {
                                updated_file_contents = match replace_value(
                                    updated_file_contents.clone(),
                                    n,
                                    FileTypes::Credentials(cr.clone()),
                                ) {
                                    Ok(v) => v,
                                    Err(e) => return Err(e),
                                }
                            }
                        }
                        FileTypes::Config(ref cf) => {
                            if n == existing_profile_line_numbers[counter] {
                                updated_file_contents = match replace_value(
                                    file_contents.clone(),
                                    n,
                                    FileTypes::Config(cf.clone()),
                                ) {
                                    Ok(v) => v,
                                    Err(e) => return Err(e),
                                }
                            } else {
                                updated_file_contents = match replace_value(
                                    updated_file_contents.clone(),
                                    n,
                                    FileTypes::Config(cf.clone()),
                                ) {
                                    Ok(v) => v,
                                    Err(e) => return Err(e),
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(updated_file_contents)
}

fn replace_value(
    file_contents: Vec<String>,
    index: usize,
    file_types: FileTypes,
) -> Result<Vec<String>, CliError> {
    let mut updated_file_contents = file_contents;

    match file_types {
        FileTypes::Credentials(cr) => {
            let token_regex = match Regex::new(r"^token\s*=\s*([\w\.-]+)\s*$") {
                Ok(r) => r,
                Err(e) => {
                    return Err(CliError {
                        msg: format!("invalid regex expression is provided, error: {}", e),
                    })
                }
            };
            let result = token_regex.replace(
                updated_file_contents[index].as_str(),
                format!("token={}\n", cr.token.as_str()),
            );
            updated_file_contents[index] = result.to_string();
            Ok(updated_file_contents)
        }
        FileTypes::Config(cf) => {
            let cache_regex = match Regex::new(r"^cache\s*=\s*([\w-]+)\s*$") {
                Ok(r) => r,
                Err(e) => {
                    return Err(CliError {
                        msg: format!("invalid regex expression is provided, error: {}", e),
                    })
                }
            };
            let result = cache_regex.replace(
                updated_file_contents[index].as_str(),
                format!("cache={}\n", cf.cache.as_str()),
            );
            updated_file_contents[index] = result.to_string();

            let ttl_regex = match Regex::new(r"^ttl\s*=\s*([\d]+)\s*$") {
                Ok(r) => r,
                Err(e) => {
                    return Err(CliError {
                        msg: format!("invalid regex expression is provided, error: {}", e),
                    })
                }
            };
            let result = ttl_regex.replace(
                updated_file_contents[index].as_str(),
                format!("ttl={}\n", cf.ttl.to_string().as_str()),
            );
            updated_file_contents[index] = result.to_string();
            Ok(updated_file_contents)
        }
    }
}
