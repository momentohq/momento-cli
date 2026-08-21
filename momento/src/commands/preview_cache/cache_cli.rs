use std::process::{exit, Command};

use crate::{error::CliError, utils::console::console_data};

use log::info;

fn build_printable_command(options: &[&[&str]], subcommand: Vec<String>) -> String {
    let separator = " \\\n  ";
    [
        vec![
            "VALKEYCLI_AUTH=$MOMENTO_API_KEY".to_string(),
            format!(
                "valkey-cli {}",
                options
                    .iter()
                    .map(|opt| opt.join(" "))
                    .collect::<Vec<_>>()
                    .join(separator)
            ),
        ],
        vec![subcommand.join(" ")],
    ]
    .concat()
    .join(separator)
}

pub async fn run_valkey_command(
    valkey_hostname: String,
    auth_token: String,
    database_name: String,
    subcommand: Vec<String>,
) -> Result<(), CliError> {
    let options: [&[&str]; 3] = [
        &["--tls"],
        &["-h", &valkey_hostname],
        &["--user", &database_name],
    ];
    let printable_command = build_printable_command(&options, subcommand.clone());
    if let Err(_) = Command::new("valkey-cli").arg("--version").output() {
        console_data!(
            "Please install valkey-cli, then try again \
             or run the command directly:\n\n{printable_command}"
        );
        return Ok(());
    };
    info!("Running Valkey command:\n{printable_command}\n");
    match Command::new("valkey-cli")
        .args(options.iter().flat_map(|flag| flag.iter()))
        .args(subcommand)
        .env("VALKEYCLI_AUTH", auth_token)
        .output()
    {
        Ok(output) => {
            info!("{output:#?}\n");
            console_data!(
                "{}",
                String::from_utf8(output.clone().stdout).unwrap_or_else(|_| String::from_utf8(
                    output.clone().stderr
                )
                .unwrap_or(format!(
                    "We couldn't parse Valkey's response:\n\n\
                     {output:#?}\n\n\
                     You can try running the command directly:\n\n{printable_command}"
                )))
            );
            match output.status.code() {
                None => Ok(()),
                Some(code) => exit(code),
            }
        }
        Err(err) => Err(CliError::new(format!("{err}"))),
    }
}
