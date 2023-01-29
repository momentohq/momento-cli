use regex::Regex;

use crate::{
    config::{Config, Credentials, FileTypes},
    error::CliError,
};

pub fn create_new_credentials_profile(profile_name: &str, credentials: Credentials) -> Vec<String> {
    vec![
        format!("[{profile_name}]"),
        format!("token={}", credentials.token),
    ]
}

pub fn create_new_config_profile(profile_name: &str, config: Config) -> Vec<String> {
    vec![
        format!("[{profile_name}]"),
        format!("cache={}", config.cache),
        format!("ttl={}", config.ttl),
    ]
}

pub fn update_profile_values(
    profile_name: &str,
    file_contents: &[impl AsRef<str>],
    file_types: FileTypes,
) -> Result<Vec<String>, CliError> {
    let existing_profile_line_numbers = match find_profile_line_numbers(file_contents) {
        None => {
            return Err(CliError {
                msg: "No profiles found!".to_string(),
            });
        }
        Some(line_numbers) => line_numbers,
    };

    let existing_profile_starting_line_num =
        find_existing_profile_start(file_contents, profile_name);

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
                                    file_contents,
                                    n,
                                    FileTypes::Credentials(cr.clone()),
                                ) {
                                    Ok(v) => v,
                                    Err(e) => return Err(e),
                                }
                            } else {
                                updated_file_contents = match replace_value(
                                    &updated_file_contents.clone(),
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
                                    file_contents,
                                    n,
                                    FileTypes::Config(cf.clone()),
                                ) {
                                    Ok(v) => v,
                                    Err(e) => return Err(e),
                                }
                            } else {
                                updated_file_contents = match replace_value(
                                    &updated_file_contents.clone(),
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
                                    file_contents,
                                    n,
                                    FileTypes::Credentials(cr.clone()),
                                ) {
                                    Ok(v) => v,
                                    Err(e) => return Err(e),
                                }
                            } else {
                                updated_file_contents = match replace_value(
                                    &updated_file_contents.clone(),
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
                                    file_contents,
                                    n,
                                    FileTypes::Config(cf.clone()),
                                ) {
                                    Ok(v) => v,
                                    Err(e) => return Err(e),
                                }
                            } else {
                                updated_file_contents = match replace_value(
                                    &updated_file_contents.clone(),
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
    file_contents: &[impl AsRef<str>],
    index: usize,
    file_types: FileTypes,
) -> Result<Vec<String>, CliError> {
    let mut updated_file_contents: Vec<String> = file_contents
        .iter()
        .map(|l| l.as_ref().to_string())
        .collect();

    match file_types {
        FileTypes::Credentials(cr) => {
            let token_regex = match Regex::new(r"^token\s*=\s*([\w\.-]*)\s*$") {
                Ok(r) => r,
                Err(e) => {
                    return Err(CliError {
                        msg: format!("invalid regex expression is provided, error: {e}"),
                    })
                }
            };
            let result = token_regex.replace(
                updated_file_contents[index].as_str(),
                format!("token={}", cr.token.as_str()),
            );
            updated_file_contents[index] = result.to_string();
            Ok(updated_file_contents)
        }
        FileTypes::Config(cf) => {
            let cache_regex = match Regex::new(r"^cache\s*=\s*([\w-]*)\s*$") {
                Ok(r) => r,
                Err(e) => {
                    return Err(CliError {
                        msg: format!("invalid regex expression is provided, error: {e}"),
                    })
                }
            };
            let result = cache_regex.replace(
                updated_file_contents[index].as_str(),
                format!("cache={}", cf.cache.as_str()),
            );
            updated_file_contents[index] = result.to_string();

            let ttl_regex = match Regex::new(r"^ttl\s*=\s*([\d]*)\s*$") {
                Ok(r) => r,
                Err(e) => {
                    return Err(CliError {
                        msg: format!("invalid regex expression is provided, error: {e}"),
                    })
                }
            };
            let result = ttl_regex.replace(
                updated_file_contents[index].as_str(),
                format!("ttl={}", cf.ttl.to_string().as_str()),
            );
            updated_file_contents[index] = result.to_string();
            Ok(updated_file_contents)
        }
    }
}

pub fn does_profile_name_exist(file_contents: &[impl AsRef<str>], profile_name: &str) -> bool {
    for line in file_contents.iter() {
        let trimmed_line = line.as_ref().to_string().replace('\n', "");
        if trimmed_line.eq(&format!("[{profile_name}]")) {
            return true;
        }
    }
    false
}

fn find_profile_line_numbers(file_contents: &[impl AsRef<str>]) -> Option<Vec<usize>> {
    let mut counter = 0;
    let mut profile_counter;
    let line_array_len = file_contents.len();
    let mut profile_start_line_num_array: Vec<usize> = Vec::new();
    while counter < line_array_len {
        let line = file_contents[counter].as_ref().trim().to_string();
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

fn find_existing_profile_start(file_contents: &[impl AsRef<str>], profile_name: &str) -> usize {
    let mut counter = 0;
    let line_array_len = file_contents.len();

    while counter < line_array_len {
        let trimmed_line = file_contents[counter]
            .as_ref()
            .to_string()
            .replace('\n', "");
        if trimmed_line.eq(&format!("[{profile_name}]")) {
            return counter;
        }
        counter += 1;
    }
    counter
}

#[cfg(test)]
mod tests {
    use crate::config::{Config, Credentials, FileTypes};
    use crate::utils::ini_config::{
        create_new_config_profile, create_new_credentials_profile, update_profile_values,
    };

    fn test_file_content(untrimmed_file_contents: &str) -> String {
        format!("{}\n", untrimmed_file_contents.trim())
    }

    #[test]
    fn create_new_credentials_profile_happy_path() {
        let profile_text = create_new_credentials_profile(
            "default",
            Credentials {
                token: "awesome-token".to_string(),
            },
        )
        .join("\n");
        let expected_text = test_file_content(
            "
[default]
token=awesome-token
        ",
        );
        assert_eq!(expected_text.trim(), profile_text);
    }

    #[test]
    fn create_new_config_profile_happy_path() {
        let profile_text = create_new_config_profile(
            "default",
            Config {
                cache: "awesome-cache".to_string(),
                ttl: 90210,
            },
        )
        .join("\n");
        let expected_text = test_file_content(
            "
[default]
cache=awesome-cache
ttl=90210
        ",
        );
        assert_eq!(expected_text.trim(), profile_text)
    }

    #[test]
    fn update_profile_values_credentials_one_existing_profile() {
        let file_contents = test_file_content(
            "
[default]
token=invalidtoken
        ",
        );
        let file_lines: Vec<&str> = file_contents.split('\n').collect();
        let creds = Credentials {
            token: "newtoken".to_string(),
        };
        let file_types = FileTypes::Credentials(creds);
        let result = update_profile_values("default", &file_lines, file_types);
        assert!(result.is_ok());
        let new_content = result.expect("d'oh").join("\n");

        let expected_content = test_file_content(
            "
[default]
token=newtoken
        ",
        );

        assert_eq!(expected_content, new_content);
    }

    #[test]
    fn update_profile_values_credentials_one_existing_profile_with_empty_token() {
        let file_contents = test_file_content(
            "
[default]
token=
        ",
        );
        let file_lines: Vec<&str> = file_contents.split('\n').collect();
        let creds = Credentials {
            token: "newtoken".to_string(),
        };
        let file_types = FileTypes::Credentials(creds);
        let result = update_profile_values("default", &file_lines, file_types);
        assert!(result.is_ok());
        let new_content = result.expect("d'oh").join("\n");

        let expected_content = test_file_content(
            "
[default]
token=newtoken
        ",
        );

        assert_eq!(expected_content, new_content);
    }

    #[test]
    fn update_profile_values_credentials_three_existing_profiles() {
        let file_contents = test_file_content(
            "
[taco]
token=invalidtoken

[default]
token=anotherinvalidtoken

[habanero]
token=spicytoken
        ",
        );
        let file_lines: Vec<&str> = file_contents.split('\n').collect();
        let creds = Credentials {
            token: "newtoken".to_string(),
        };
        let file_types = FileTypes::Credentials(creds);
        let result = update_profile_values("default", &file_lines, file_types);
        assert!(result.is_ok());
        let new_content = result.expect("d'oh").join("\n");

        let expected_content = test_file_content(
            "
[taco]
token=invalidtoken

[default]
token=newtoken

[habanero]
token=spicytoken
        ",
        );

        assert_eq!(expected_content, new_content);
    }

    #[test]
    fn update_profile_values_config_one_existing_profile() {
        let file_contents = test_file_content(
            "
[default]
cache=default-cache
ttl=600
        ",
        );
        let file_lines: Vec<&str> = file_contents.split('\n').collect();
        let config = Config {
            cache: "new-cache".to_string(),
            ttl: 90210,
        };
        let file_types = FileTypes::Config(config);
        let result = update_profile_values("default", &file_lines, file_types);
        assert!(result.is_ok());
        let new_content = result.expect("d'oh").join("\n");

        let expected_content = test_file_content(
            "
[default]
cache=new-cache
ttl=90210
        ",
        );

        assert_eq!(expected_content, new_content);
    }

    #[test]
    fn update_profile_values_config_three_existing_profiles() {
        let file_contents = test_file_content(
            "
[taco]
cache=yummy-cache
ttl=600

[default]
cache=default-cache
ttl=600

[habanero]
cache=spicy-cache
ttl=600
        ",
        );
        let file_lines: Vec<&str> = file_contents.split('\n').collect();
        let config = Config {
            cache: "new-cache".to_string(),
            ttl: 90210,
        };
        let file_types = FileTypes::Config(config);
        let result = update_profile_values("default", &file_lines, file_types);
        assert!(result.is_ok());
        let new_content = result.expect("d'oh").join("\n");

        let expected_content = test_file_content(
            "
[taco]
cache=yummy-cache
ttl=600

[default]
cache=new-cache
ttl=90210

[habanero]
cache=spicy-cache
ttl=600
        ",
        );

        assert_eq!(expected_content, new_content);
    }
}
