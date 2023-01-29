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
    let momento_home = get_momento_dir()?;
    Ok(format!("{momento_home}/credentials"))
}

pub fn get_config_file_path() -> Result<String, CliError> {
    let momento_home = get_momento_dir()?;
    Ok(format!("{momento_home}/config"))
}

pub fn get_momento_dir() -> Result<String, CliError> {
    let home = home_dir().ok_or_else(|| CliError {
        msg: "could not find home dir".to_string(),
    })?;
    Ok(format!("{}/.momento", home.display()))
}

pub async fn open_file(path: &str) -> Result<File, CliError> {
    let res = File::open(path).await;
    match res {
        Ok(f) => {
            debug!("opened file {path}");
            Ok(f)
        }
        Err(e) => Err(CliError {
            msg: format!("failed to create file {path}, error: {e}"),
        }),
    }
}

pub async fn read_ini_file(path: &str) -> Result<Ini, CliError> {
    let mut config = Ini::new_cs();
    match config.load(path) {
        Ok(_) => Ok(config),
        Err(e) => Err(CliError {
            msg: format!("failed to read file: {e}"),
        }),
    }
}

pub async fn read_file_contents(file: File) -> Result<Vec<String>, CliError> {
    let reader = BufReader::new(file);
    let mut contents = reader.lines();
    // Put each line read from file to a vector
    let mut file_contents: Vec<String> = vec![];
    while let Some(line) = contents.next_line().await.map_err(|e| CliError {
        msg: format!("could not read next line: {e:?}"),
    })? {
        file_contents.push(line.to_string());
    }
    Ok(file_contents)
}

pub async fn create_file(path: &str) -> Result<(), CliError> {
    let res = File::create(path).await;
    match res {
        Ok(_) => {
            debug!("created file {}", path);
            Ok(())
        }
        Err(e) => Err(CliError {
            msg: format!("failed to create file {path}, error: {e}"),
        }),
    }
}

pub async fn write_to_file(path: &str, file_contents: String) -> Result<(), CliError> {
    let mut file = match fs::File::create(path).await {
        Ok(f) => f,
        Err(e) => {
            return Err(CliError {
                msg: format!("failed to write to file {path}, error: {e}"),
            })
        }
    };

    // Write to file

    match file.write(file_contents.as_bytes()).await {
        Ok(_) => {}
        Err(e) => {
            return Err(CliError {
                msg: format!("failed to write to file {path}, error: {e}"),
            })
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
            return Err(CliError {
                msg: format!("failed to write prompt to stdout: {e}"),
            })
        }
    };
    match stdout.flush().await {
        Ok(_) => debug!("flushed stdout"),
        Err(e) => {
            return Err(CliError {
                msg: format!("failed to flush stdout: {e}"),
            })
        }
    };
    let stdin = io::stdin();
    let mut buffer = String::new();
    let mut reader = BufReader::new(stdin);
    match reader.read_line(&mut buffer).await {
        Ok(_) => debug!("read line from stdin"),
        Err(e) => {
            return Err(CliError {
                msg: format!("failed to read line from stdin: {e}"),
            })
        }
    };

    let input = buffer.as_str().trim().to_string();
    if input.is_empty() {
        return Ok(default_value.to_string());
    }
    Ok(input)
}
