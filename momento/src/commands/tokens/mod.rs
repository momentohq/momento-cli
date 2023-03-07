use momento::requests::generate_api_token_request::TokenExpiry;

use crate::{
    error::CliError,
    utils::client::{get_momento_client, interact_with_momento, print_whatever_this_is_as_json},
};

pub async fn generate_api_token(
    auth_token: String,
    endpoint: Option<String>,
    expiry: TokenExpiry,
) -> Result<(), CliError> {
    let mut client = get_momento_client(auth_token, endpoint).await?;
    print_whatever_this_is_as_json(
        &interact_with_momento("generating...", client.generate_api_token(expiry)).await?,
    );
    Ok(())
}
