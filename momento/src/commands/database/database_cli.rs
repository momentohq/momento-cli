use super::utils::{DatabaseError, DatabaseResponse};
use crate::{error::CliError, utils::console::console_data};

use http::Method;
use log::{info, warn};
use reqwest;
use serde_json;

pub async fn create_database(
    endpoint: String,
    auth_token: String,
    pool_name: String,
    database_name: String,
) -> Result<(), CliError> {
    let request_url = format!("{endpoint}/database/{database_name}");
    let data = serde_json::json!({
        "pool_name": pool_name
    })
    .to_string();

    let req_client = reqwest::Client::builder().build()?;
    let response = req_client
        .request(Method::POST, &request_url)
        .body(data)
        .header("authorization", &auth_token)
        .header("content-type", "application/json")
        .send()
        .await?;
    let status = response.status();
    if status.is_success() {
        let response_text = response.text().await?;
        match serde_json::from_str::<DatabaseResponse>(response_text.as_str()) {
            Ok(database) => {
                info!("Response sent back by {database_name} database creation:\n{database:#?}");
                console_data!(
                    "Database created! Name: {}, Pool: {}",
                    database.name,
                    database.pool_name
                );
            }
            Err(err) => {
                warn!(
                    "Can't parse response from {database_name} database creation:\n{response_text}\n{err}"
                );
                console_data!("Database created! {response_text}");
            }
        };
        Ok(())
    } else {
        let error_text = response.text().await?;
        Err(CliError::new(if error_text.is_empty() {
            format!("{status}")
        } else {
            let error_message = match serde_json::from_str::<DatabaseError>(error_text.as_str()) {
                Ok(error) => error
                    .detail
                    .unwrap_or(error.message.unwrap_or(error_text.clone())),
                Err(_) => error_text.clone(),
            };
            format!("{status}: {error_message}")
        })
        .with_details(error_text))
    }
}
