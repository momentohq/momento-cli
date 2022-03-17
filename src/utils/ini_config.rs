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
    profile_line_num_array: Vec<usize>,
    line_num_of_existing_profile: usize,
    line_array: &mut [String],
    file_types: FileTypes,
) {
    let num_of_profiles = profile_line_num_array.len();
    let line_array_len = line_array.len();
    for (counter, line_num) in profile_line_num_array.iter().enumerate() {
        #[allow(clippy:needless_range_loop)]
        if line_num_of_existing_profile == *line_num {
            // Case where profile_name is the only or last item in profile_line_num_array
            if counter == num_of_profiles - 1 {
                #[allow(clippy::needless_range_loop)]
                for n in *line_num..line_array_len {
                    match file_types {
                        FileTypes::Credentials(ref cr) => {
                            replace_value(line_array, n, FileTypes::Credentials(cr.clone()))
                        }
                        FileTypes::Config(ref cf) => {
                            replace_value(line_array, n, FileTypes::Config(cf.clone()))
                        }
                    }
                }
            } else {
                // Case where profile_name is at the beginning or at the middle of profile_line_num_array
                #[allow(clippy::needless_range_loop)]
                for n in profile_line_num_array[counter]..profile_line_num_array[counter + 1] {
                    match file_types {
                        FileTypes::Credentials(ref cr) => {
                            replace_value(line_array, n, FileTypes::Credentials(cr.clone()))
                        }
                        FileTypes::Config(ref cf) => {
                            replace_value(line_array, n, FileTypes::Config(cf.clone()))
                        }
                    }
                }
            }
        }
    }
}

fn replace_value(line_array: &mut [String], index: usize, file_types: FileTypes) {
    match file_types {
        FileTypes::Credentials(cr) => {
            // Check if line is not a comment or profile
            if !line_array[index].starts_with('#') && !line_array[index].starts_with('[') {
                let line_len = line_array[index].len();
                // Replace value after "token="
                line_array[index]
                    .replace_range(AFTER_TOKEN_INDEX..line_len, &format!("{}\n", &cr.token));
            }
        }
        FileTypes::Config(cf) => {
            // Check if line is not a comment or profile and for cache
            if !line_array[index].starts_with('#')
                && !line_array[index].starts_with('[')
                && line_array[index].starts_with('c')
            {
                let line_len = line_array[index].len();
                // Replace value after "cache="
                line_array[index]
                    .replace_range(AFTER_CACHE_INDEX..line_len, &format!("{}\n", &cf.cache));
            }
            // Check if line is not a comment or profile and for ttl
            if !line_array[index].starts_with('#')
                && !line_array[index].starts_with('[')
                && line_array[index].starts_with('t')
            {
                let line_len = line_array[index].len();
                // Replace value after "ttl="
                line_array[index].replace_range(
                    AFTER_TTL_INDEX..line_len,
                    &format!("{}\n", &cf.ttl.to_string()),
                );
            }
        }
    }
}
