use crate::error::CliError;

pub async fn signup_decommissioned() -> Result<(), CliError> {
    return Err(CliError {
        msg: r"This command has been decommissioned!
Please go to the console to sign up for Momento and generate a token:
https://console.gomomento.com"
            .to_string(),
    });
}
