use momento::requests::generate_api_token_request::TokenExpiry;

use crate::{
    error::CliError,
    utils::client::{get_momento_client, interact_with_momento, print_whatever_this_is_as_json},
};

pub async fn generate_api_token(
    auth_token: String,
    endpoint: Option<String>,
    never_expire: bool,
    valid_for: Option<String>,
) -> Result<(), CliError> {
    let expiry = if never_expire {
        TokenExpiry::Never {}
    } else {
        let seconds = valid_for
            .expect("oneof --valid-for, or --never-expire, must be set")
            .as_str()
            .parse::<humantime::Duration>()
            .expect("unable to parse valid_for duration")
            .as_secs();
        TokenExpiry::Expires {
            valid_for_seconds: seconds as u32,
        }
    };
    let mut client = get_momento_client(auth_token, endpoint).await?;
    print_whatever_this_is_as_json(
        &interact_with_momento("generating api token...", client.generate_api_token(expiry))
            .await?,
    );
    Ok(())
}
