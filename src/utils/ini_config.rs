use configparser::ini::Ini;

use crate::{
    config::{Config, Credentials},
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
        Ok(_) => return Ok(()),
        Err(e) => return Err(e),
    };
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
        Ok(_) => return Ok(()),
        Err(e) => return Err(e),
    };
}

pub fn update_token_value(
    profile_line_num_array: Vec<usize>,
    line_num_of_existing_profile: usize,
    line_array: &mut Vec<String>,
    credentials: Credentials,
) {
    let mut counter = 0;
    let num_of_profiles = profile_line_num_array.len();
    let line_array_len = line_array.len();
    for line_num in profile_line_num_array.iter() {
        if line_num_of_existing_profile == *line_num {
            // Case where profile_name is the last item in profile_line_num_array
            if counter == num_of_profiles - 1 {
                for n in *line_num..line_array_len {
                    // Check if line is not a comment or profile
                    if !line_array[n].starts_with('#') && !line_array[n].starts_with('[') {
                        let line_len = line_array[n].len();
                        // Replace value after "token="
                        line_array[n].replace_range(
                            AFTER_TOKEN_INDEX..line_len,
                            &format!("{}\n", &credentials.token),
                        );
                    }
                }
            } else {
                // Case where profile_name is at the beginning or at the middle of profile_line_num_array
                for n in profile_line_num_array[counter]..profile_line_num_array[counter + 1] {
                    // Check if line is not a comment or profile
                    if !line_array[n].starts_with('#') && !line_array[n].starts_with('[') {
                        let line_len = line_array[n].len();
                        // Replace value after "token="
                        line_array[n].replace_range(
                            AFTER_TOKEN_INDEX..line_len,
                            &format!("{}\n", &credentials.token),
                        );
                    }
                }
            }
        }
        counter += 1;
    }
}

pub fn update_cache_ttl_value(
    profile_line_num_array: Vec<usize>,
    line_num_of_existing_profile: usize,
    line_array: &mut Vec<String>,
    config: Config,
) {
    let mut counter = 0;
    let num_of_profiles = profile_line_num_array.len();
    let line_array_len = line_array.len();
    for line_num in profile_line_num_array.iter() {
        if line_num_of_existing_profile == *line_num {
            // Case where profile_name is the last item in profile_line_num_array
            if counter == num_of_profiles - 1 {
                for n in *line_num..line_array_len {
                    // Check if line is not a comment or profile and for cache
                    if !line_array[n].starts_with('#')
                        && !line_array[n].starts_with('[')
                        && line_array[n].starts_with('c')
                    {
                        let line_len = line_array[n].len();
                        // Replace value after "cache="
                        line_array[n].replace_range(
                            AFTER_CACHE_INDEX..line_len,
                            &format!("{}\n", &config.cache),
                        );
                    }
                    // Check if line is not a comment or profile and for ttl
                    if !line_array[n].starts_with('#')
                        && !line_array[n].starts_with('[')
                        && line_array[n].starts_with('t')
                    {
                        let line_len = line_array[n].len();
                        // Replace value after "ttl="
                        line_array[n].replace_range(
                            AFTER_TTL_INDEX..line_len,
                            &format!("{}\n", &config.ttl.to_string()),
                        );
                    }
                }
            } else {
                // Case where profile_name is at the beginning or at the middle of profile_line_num_array
                for n in profile_line_num_array[counter]..profile_line_num_array[counter + 1] {
                    // Check if line is not a comment or profile and for cache
                    if !line_array[n].starts_with('#')
                        && !line_array[n].starts_with('[')
                        && line_array[n].starts_with('c')
                    {
                        let line_len = line_array[n].len();
                        // Replace value after "cache="
                        line_array[n].replace_range(
                            AFTER_CACHE_INDEX..line_len,
                            &format!("{}\n", &config.cache),
                        );
                    }
                    // Check if line is not a comment or profile and for ttl
                    if !line_array[n].starts_with('#')
                        && !line_array[n].starts_with('[')
                        && line_array[n].starts_with('t')
                    {
                        let line_len = line_array[n].len();
                        // Replace value after "ttl="
                        line_array[n].replace_range(
                            AFTER_TTL_INDEX..line_len,
                            &format!("{}\n", &config.ttl.to_string()),
                        );
                    }
                }
            }
        }
        counter += 1;
    }
}
