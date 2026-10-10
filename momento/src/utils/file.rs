use std::path::Path;

use crate::config::ENV_VAR_NAME_MOMENTO_CONFIG_DIR;
use configparser::ini::Ini;
use home::home_dir;
use log::debug;
use tokio::{
    fs::{self, File},
    io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader},
};

use crate::error::CliError;

// FIXME All of this stuff should be using pathbuf and not concatenating strings with /'s...
pub fn get_credentials_file_path() -> Result<String, CliError> {
    let momento_home = get_momento_config_dir()?;
    Ok(format!("{momento_home}/credentials"))
}

pub fn get_config_file_path() -> Result<String, CliError> {
    let momento_home = get_momento_config_dir()?;
    Ok(format!("{momento_home}/config"))
}

pub fn get_momento_config_dir() -> Result<String, CliError> {
    let env_var = std::env::var(ENV_VAR_NAME_MOMENTO_CONFIG_DIR);

    if let Ok(val) = env_var {
        return Ok(val);
    }
    // If the env var isn't set we default to ~/.momento
    let home = home_dir().ok_or_else(|| CliError::new("could not find home dir"))?;
    Ok(format!("{}/.momento", home.display()))
}

async fn open_file(path: &str) -> Result<File, CliError> {
    let res = File::open(path).await;
    match res {
        Ok(f) => {
            debug!("opened file {path}");
            Ok(f)
        }
        Err(e) => Err(CliError::new(format!(
            "failed to create file {path}, error: {e}"
        ))),
    }
}

#[cfg(target_os = "linux")]
async fn set_file_read_write(path: &str) -> Result<(), CliError> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = match fs::metadata(path).await {
        Ok(p) => p,
        Err(e) => return Err(CliError::new(format!("failed to get file permissions {e}"))),
    }
    .permissions();
    perms.set_mode(0o600);
    match fs::set_permissions(path, perms).await {
        Ok(_) => Ok(()),
        Err(e) => Err(CliError::new(format!("failed to set file permissions {e}"))),
    }
}

#[cfg(target_os = "macos")]
async fn set_file_read_write(path: &str) -> Result<(), CliError> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = match fs::metadata(path).await {
        Ok(p) => p,
        Err(e) => return Err(CliError::new(format!("failed to get file permissions {e}"))),
    }
    .permissions();
    perms.set_mode(0o600);
    match fs::set_permissions(path, perms).await {
        Ok(_) => Ok(()),
        Err(e) => Err(CliError::new(format!("failed to set file permissions {e}"))),
    }
}

#[cfg(target_os = "windows")]
async fn set_file_read_write(path: &str) -> Result<(), CliError> {
    let mut perms = match fs::metadata(path).await {
        Ok(p) => p,
        Err(e) => return Err(CliError::new(format!("failed to get file permissions {e}"))),
    }
    .permissions();
    perms.set_readonly(false);
    match fs::set_permissions(path, perms).await {
        Ok(_) => Ok(()),
        Err(e) => Err(CliError::new(format!("failed to set file permissions {e}"))),
    }
}

pub async fn ensure_file_exists_and_get_contents(path: &str) -> Result<Vec<String>, CliError> {
    if !Path::new(path).exists() {
        match create_file(path).await {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
    }
    // explicitly allowing read/write access to the file
    set_file_read_write(path).await?;

    let file = open_file(path).await?;
    read_file_contents(file).await
}

pub async fn read_ini_file(path: &str) -> Result<Ini, CliError> {
    let mut config = Ini::new_cs();
    match config.load(path) {
        Ok(_) => Ok(config),
        Err(e) => Err(CliError::new(format!("failed to read file: {e}"))),
    }
}

async fn read_file_contents(file: File) -> Result<Vec<String>, CliError> {
    let reader = BufReader::new(file);
    let mut contents = reader.lines();
    // Put each line read from file to a vector
    let mut file_contents: Vec<String> = vec![];
    while let Some(line) = contents
        .next_line()
        .await
        .map_err(|e| CliError::new(format!("could not read next line: {e:?}")))?
    {
        file_contents.push(line.to_string());
    }
    Ok(file_contents)
}

pub fn trim_file_contents(lines: Vec<String>) -> Vec<String> {
    // TODO inefficient, can optimize later if necessary
    let content = lines.join("\n");
    let trimmed = content.trim();
    if trimmed.is_empty() {
        vec![]
    } else {
        trimmed.split('\n').map(|line| line.to_string()).collect()
    }
}

async fn create_file(path: &str) -> Result<(), CliError> {
    let res = File::create(path).await;
    match res {
        Ok(_) => {
            debug!("created file {}", path);
            Ok(())
        }
        Err(e) => Err(CliError::new(format!(
            "failed to create file {path}, error: {e}"
        ))),
    }
}

pub async fn write_to_file(path: &str, lines: Vec<String>) -> Result<(), CliError> {
    let mut file = match fs::File::create(path).await {
        Ok(f) => f,
        Err(e) => {
            return Err(CliError::new(format!(
                "failed to write to file {path}, error: {e}"
            )))
        }
    };

    let file_contents = format!(
        "{}\n", // ensure a single trailing newline
        lines.join("\n").trim_end()
    );

    // Write to file

    match file.write(file_contents.as_bytes()).await {
        Ok(_) => {}
        Err(e) => {
            return Err(CliError::new(format!(
                "failed to write to file {path}, error: {e}"
            )))
        }
    };

    Ok(())
}

pub async fn prompt_user_for_input(
    prompt: &str,
    default_value: &str,
    is_secret: bool,
) -> Result<String, CliError> {
    let mut stdout = io::stdout();

    let formatted_prompt = if default_value.is_empty() {
        format!("{prompt}: ")
    } else if is_secret {
        format!("{prompt} [****]: ")
    } else {
        format!("{prompt} [{default_value}]: ")
    };

    match stdout.write(formatted_prompt.as_bytes()).await {
        Ok(_) => debug!("wrote prompt '{}' to stdout", formatted_prompt),
        Err(e) => {
            return Err(CliError::new(format!(
                "failed to write prompt to stdout: {e}"
            )))
        }
    };
    match stdout.flush().await {
        Ok(_) => debug!("flushed stdout"),
        Err(e) => return Err(CliError::new(format!("failed to flush stdout: {e}"))),
    };
    let stdin = io::stdin();
    let mut buffer = String::new();
    let mut reader = BufReader::new(stdin);
    match reader.read_line(&mut buffer).await {
        Ok(_) => debug!("read line from stdin"),
        Err(e) => {
            return Err(CliError::new(format!(
                "failed to read line from stdin: {e}"
            )))
        }
    };

    let input = buffer.as_str().trim().to_string();
    if input.is_empty() {
        return Ok(default_value.to_string());
    }
    Ok(input)
}
