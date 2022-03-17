use configparser::ini::Ini;

use crate::{
    config::{Config, Credentials, FileTypes},
    error::CliError,
    utils::file::ini_write_to_file,
};

const AFTER_TOKEN_INDEX: usize = 6;
const AFTER_CACHE_INDEX: usize = 6;
const AFTER_TTL_INDEX: usize = 4;

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
) -> Vec<String> {
    let num_of_profiles = existing_profile_line_numbers.len();
    let file_contents_len = file_contents.len();
    let mut updated_file_contents: Vec<String> = Vec::new();
    for (counter, line_num) in existing_profile_line_numbers.iter().enumerate() {
        #[allow(clippy:needless_range_loop)]
        if existing_profile_starting_line_num == *line_num {
            // Case where profile_name is the only or last item in existing_profile_line_numbers
            if counter == num_of_profiles - 1 {
                #[allow(clippy::needless_range_loop)]
                for n in *line_num..file_contents_len {
                    match file_types {
                        FileTypes::Credentials(ref cr) => {
                            if n == *line_num {
                                updated_file_contents = replace_value(
                                    file_contents.clone(),
                                    n,
                                    FileTypes::Credentials(cr.clone()),
                                )
                            } else {
                                updated_file_contents = replace_value(
                                    updated_file_contents.clone(),
                                    n,
                                    FileTypes::Credentials(cr.clone()),
                                )
                            }
                        }
                        FileTypes::Config(ref cf) => {
                            if n == *line_num {
                                updated_file_contents = replace_value(
                                    file_contents.clone(),
                                    n,
                                    FileTypes::Config(cf.clone()),
                                )
                            } else {
                                updated_file_contents = replace_value(
                                    updated_file_contents.clone(),
                                    n,
                                    FileTypes::Config(cf.clone()),
                                )
                            }
                        }
                    }
                }
            } else {
                // Case where profile_name is at the beginning or at the middle of existing_profile_line_numbers
                #[allow(clippy::needless_range_loop)]
                for n in existing_profile_line_numbers[counter]
                    ..existing_profile_line_numbers[counter + 1]
                {
                    match file_types {
                        FileTypes::Credentials(ref cr) => {
                            if n == existing_profile_line_numbers[counter] {
                                updated_file_contents = replace_value(
                                    file_contents.clone(),
                                    n,
                                    FileTypes::Credentials(cr.clone()),
                                )
                            } else {
                                updated_file_contents = replace_value(
                                    updated_file_contents.clone(),
                                    n,
                                    FileTypes::Credentials(cr.clone()),
                                )
                            }
                        }
                        FileTypes::Config(ref cf) => {
                            if n == existing_profile_line_numbers[counter] {
                                updated_file_contents = replace_value(
                                    file_contents.clone(),
                                    n,
                                    FileTypes::Config(cf.clone()),
                                )
                            } else {
                                updated_file_contents = replace_value(
                                    updated_file_contents.clone(),
                                    n,
                                    FileTypes::Config(cf.clone()),
                                )
                            }
                        }
                    }
                }
            }
        }
    }
    updated_file_contents
}

fn replace_value(file_contents: Vec<String>, index: usize, file_types: FileTypes) -> Vec<String> {
    let mut updated_file_contents = file_contents;
    match file_types {
        FileTypes::Credentials(cr) => {
            // Check if line is not a comment or profile
            if !updated_file_contents[index].starts_with('#')
                && !updated_file_contents[index].starts_with('[')
            {
                let line_len = updated_file_contents[index].len();
                // Replace value after "token="
                updated_file_contents[index]
                    .replace_range(AFTER_TOKEN_INDEX..line_len, &format!("{}\n", &cr.token));
            }
            updated_file_contents
        }
        FileTypes::Config(cf) => {
            // Check if line is not a comment or profile and for cache
            if !updated_file_contents[index].starts_with('#')
                && !updated_file_contents[index].starts_with('[')
                && updated_file_contents[index].starts_with('c')
            {
                let line_len = updated_file_contents[index].len();
                // Replace value after "cache="
                updated_file_contents[index]
                    .replace_range(AFTER_CACHE_INDEX..line_len, &format!("{}\n", &cf.cache));
            }
            // Check if line is not a comment or profile and for ttl
            if !updated_file_contents[index].starts_with('#')
                && !updated_file_contents[index].starts_with('[')
                && updated_file_contents[index].starts_with('t')
            {
                let line_len = updated_file_contents[index].len();
                // Replace value after "ttl="
                updated_file_contents[index].replace_range(
                    AFTER_TTL_INDEX..line_len,
                    &format!("{}\n", &cf.ttl.to_string()),
                );
            }
            updated_file_contents
        }
    }
}
