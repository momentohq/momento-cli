use crate::utils::ini_config::update_config_profile;
use crate::{
    config::Config,
    error::CliError,
    utils::{
        file::{
            ensure_file_exists_and_get_contents, get_config_file_path, trim_file_contents,
            write_to_file,
        },
        ini_config::{create_new_config_profile, does_profile_name_exist},
    },
};

pub async fn write_default_config(profile_name: &str, config: Config) -> Result<(), CliError> {
    let config_file_path = get_config_file_path()?;
    let config_file_contents = ensure_file_exists_and_get_contents(&config_file_path).await?;
    let new_config_file_contents =
        add_or_update_profile_config(profile_name, config.clone(), config_file_contents)?;
    write_to_file(&config_file_path, new_config_file_contents).await?;
    Ok(())
}

fn add_or_update_profile_config(
    profile_name: &str,
    config: Config,
    file_contents: Vec<String>,
) -> Result<Vec<String>, CliError> {
    let trimmed_file_contents = trim_file_contents(file_contents);
    // If profile_name does not exist yet, add new profile and token value
    if !does_profile_name_exist(&trimmed_file_contents, profile_name) {
        Ok(add_new_config_profile(
            config,
            profile_name,
            trimmed_file_contents,
        ))
    } else {
        // If profile_name already exists, update token value
        update_config_profile(profile_name, &trimmed_file_contents, config)
    }
}

fn add_new_config_profile(
    config: Config,
    profile_name: &str,
    current_file_content: Vec<String>,
) -> Vec<String> {
    let new_profile = create_new_config_profile(profile_name, config);
    if current_file_content.is_empty() {
        new_profile
    } else {
        [current_file_content, vec!["\n".to_string()], new_profile].concat()
    }
}
