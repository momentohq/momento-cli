use super::utils::call_database_api;
use crate::commands::utils::MomentoHttpResponse::{Parsed, Unparseable};
use crate::{error::CliError, utils::console::console_data};

use http::Method;
use serde_json;

pub async fn create_database(
    endpoint: String,
    auth_token: String,
    pool_name: String,
    database_name: String,
) -> Result<(), CliError> {
    match call_database_api(
        Method::POST,
        endpoint,
        auth_token,
        database_name,
        Some(serde_json::json!({
            "pool_name": pool_name
        })),
    )
    .await?
    {
        Parsed(database) => {
            console_data!(
                "Creating database! Name: {}, Pool: {}",
                database.name,
                database.pool_name,
            );
        }
        Unparseable(response_text) => {
            console_data!("Creating database! {response_text}");
        }
    };
    Ok(())
}
