use crate::error::CliError;

pub async fn signup_deprecated() -> Result<(), CliError> {
    return Err(CliError { msg:"This command has been deprecated and removed! \nPlease go to the console to sign up for Momento and generate a token: \n https://console.gomomento.com".to_string() });
}
