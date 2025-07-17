use lazy_static::lazy_static;
use regex::Regex;

use crate::{
    config::{Config, Credentials},
    error::CliError,
};

lazy_static! {
    static ref REGEX_PROFILE_HEADER: Regex =
        Regex::new("^\\s*\\[[^\\]]+\\]\\s*$").expect("Unable to compile profile header regex");
    static ref REGEX_CONFIG_FILE_CACHE_SETTING: Regex = Regex::new(r"^cache\s*=\s*([\w-]*)\s*$")
        .expect("Unable to compile config file cache setting regex");
    static ref REGEX_CONFIG_FILE_TTL_SETTING: Regex = Regex::new(r"^ttl\s*=\s*([\d]*)\s*$")
        .expect("Unable to compile config file ttl setting regex");
    static ref REGEX_CREDS_FILE_TOKEN_SETTING: Regex = Regex::new(r"^token\s*=\s*([\w\.-=]*)\s*$")
        .expect("Unable to compile creds file token regex");
}

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

pub fn update_credentials_profile(
    profile_name: &str,
    file_contents: &[impl AsRef<str>],
    credentials: Credentials,
) -> Result<Vec<String>, CliError> {
    let (profile_start_line, profile_end_line) =
        find_line_numbers_for_profile(file_contents, profile_name);
    let mut updated_file_contents: Vec<String> = file_contents
        .iter()
        .map(|l| l.as_ref().to_string())
        .collect();
    for l in updated_file_contents
        .iter_mut()
        .take(profile_end_line)
        .skip(profile_start_line)
    {
        *l = replace_credentials_value(l, &credentials)
    }
    Ok(updated_file_contents)
}

pub fn update_config_profile<T: AsRef<str>>(
    profile_name: &str,
    file_contents: &[T],
    config: Config,
) -> Result<Vec<String>, CliError> {
    let (profile_start_line, profile_end_line) =
        find_line_numbers_for_profile(file_contents, profile_name);
    let mut updated_file_contents: Vec<String> = file_contents
        .iter()
        .map(|l| l.as_ref().to_string())
        .collect();
    for l in updated_file_contents
        .iter_mut()
        .take(profile_end_line)
        .skip(profile_start_line)
    {
        *l = replace_config_value(l, &config)
    }
    Ok(updated_file_contents)
}

fn replace_credentials_value(existing_line: &str, credentials: &Credentials) -> String {
    let line_with_updated_token = REGEX_CREDS_FILE_TOKEN_SETTING
        .replace(existing_line, format!("token={}", credentials.token));

    line_with_updated_token.to_string()
}

fn replace_config_value(existing_line: &str, config: &Config) -> String {
    let line_with_updated_cache =
        REGEX_CONFIG_FILE_CACHE_SETTING.replace(existing_line, format!("cache={}", config.cache));

    let line_with_updated_ttl = REGEX_CONFIG_FILE_TTL_SETTING.replace(
        line_with_updated_cache.as_ref(),
        format!("ttl={}", config.ttl),
    );

    line_with_updated_ttl.to_string()
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

fn find_line_numbers_for_profile(
    file_contents: &[impl AsRef<str>],
    profile_name: &str,
) -> (usize, usize) {
    let mut counter = 0;
    let mut start_line: usize = 0;
    let mut end_line: usize = file_contents.len();

    let mut lines_iter = file_contents.iter();
    let expected_profile_line = format!("[{profile_name}]");

    loop {
        let line = lines_iter.next();
        match line {
            None => {
                break;
            }
            Some(l) => {
                if *(l.as_ref()) == expected_profile_line {
                    start_line = counter;
                    break;
                }
            }
        }
        counter += 1;
    }

    loop {
        counter += 1;
        let line = lines_iter.next();
        match line {
            None => {
                break;
            }
            Some(l) => {
                if is_profile_header_line(l.as_ref()) {
                    end_line = counter;
                    break;
                }
            }
        }
    }

    (start_line, end_line)
}

fn is_profile_header_line(line: &str) -> bool {
    REGEX_PROFILE_HEADER.is_match(line)
}

#[cfg(test)]
mod tests {
    use crate::config::{Config, Credentials};
    use crate::utils::ini_config::{
        create_new_config_profile, create_new_credentials_profile, update_config_profile,
        update_credentials_profile,
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
    fn update_credentials_profile_values_one_existing_profile() {
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
        let result = update_credentials_profile("default", &file_lines, creds);
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
    fn update_credentials_profile_values_one_existing_profile_with_empty_token() {
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
        let result = update_credentials_profile("default", &file_lines, creds);
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
    fn update_credentials_profile_values_three_existing_profiles() {
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
        let result = update_credentials_profile("default", &file_lines, creds);
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
        let result = update_config_profile("default", &file_lines, config);
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
        let result = update_config_profile("default", &file_lines, config);
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
