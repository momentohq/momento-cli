use super::utils::{call_database_api, call_database_delete_api, call_database_list_api};
use crate::commands::database::utils::ListDatabasesResponse;
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
                "Creating database! Name: {}, Capacity Pool: {}",
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

pub async fn describe_database(
    endpoint: String,
    auth_token: String,
    name: String,
) -> Result<(), CliError> {
    match call_database_api(Method::GET, endpoint, auth_token, name, None).await? {
        Parsed(database) => {
            console_data!(
                "Your database:\nName: {}, Capacity Pool: {}",
                database.name,
                database.pool_name
            );
        }
        Unparseable(response_text) => {
            console_data!("Your database:\n{response_text}");
        }
    };
    Ok(())
}

pub async fn delete_database(
    endpoint: String,
    auth_token: String,
    database_name: String,
) -> Result<(), CliError> {
    let response_text =
        call_database_delete_api(endpoint, auth_token, database_name.clone()).await?;
    console_data!("Deleting database {database_name}! {response_text}");
    Ok(())
}

pub async fn list_databases(endpoint: String, auth_token: String) -> Result<(), CliError> {
    let response = call_database_list_api(endpoint, auth_token).await?;
    match response {
        Parsed(ListDatabasesResponse {
            databases: databases_list,
        }) => {
            if databases_list.is_empty() {
                console_data!("No databases found");
            } else {
                console_data!("Databases:");
                databases_list.iter().for_each(|database| {
                    console_data!(
                        "\nName: {}, Capacity Pool: {}",
                        database.name,
                        database.pool_name
                    );
                });
            }
        }
        Unparseable(response_text) => {
            console_data!("Listing databases:\n{response_text}");
        }
    };
    Ok(())
}
