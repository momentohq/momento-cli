use super::utils::{call_database_api, call_database_delete_api, call_database_list_api, Database};
use crate::commands::database::utils::{print_valkey_cli_sample, ListDatabasesResponse};
use crate::commands::utils::{
    CustomerMetricsConfig,
    MomentoHttpResponse::{Parsed, Unparseable},
};
use crate::{error::CliError, utils::console::console_data};

use http::Method;
use serde_json;

pub async fn create_database(
    api_endpoint: String,
    valkey_hostname: String,
    auth_token: String,
    pool_name: String,
    database_name: String,
    metrics_config: Option<CustomerMetricsConfig>,
) -> Result<(), CliError> {
    let data = serde_json::to_value(Database {
        pool_name,
        metrics_config,
    })?;
    match call_database_api(
        Method::POST,
        api_endpoint,
        auth_token,
        database_name.clone(),
        Some(data),
    )
    .await?
    {
        Parsed(database) => {
            console_data!("Creating database!\n\n{database}");
        }
        Unparseable(response_text) => {
            console_data!("Creating database!");
            if !response_text.is_empty() {
                console_data!("\n\n{response_text}");
            }
        }
    };
    print_valkey_cli_sample(valkey_hostname, &database_name);
    Ok(())
}

pub async fn describe_database(
    api_endpoint: String,
    valkey_hostname: String,
    auth_token: String,
    name: String,
) -> Result<(), CliError> {
    let database_name =
        match call_database_api(Method::GET, api_endpoint, auth_token, name, None).await? {
            Parsed(database) => {
                console_data!("Your database:\n\n{database}");
                database.name
            }
            Unparseable(response_text) => {
                console_data!("Your database:");
                if !response_text.is_empty() {
                    console_data!("\n\n{response_text}");
                }
                "<DATABASE NAME>".to_string()
            }
        };
    print_valkey_cli_sample(valkey_hostname, &database_name);
    Ok(())
}

pub async fn update_database(
    api_endpoint: String,
    auth_token: String,
    database_name: String,
    metrics_config: Option<CustomerMetricsConfig>,
) -> Result<(), CliError> {
    let data = serde_json::json!({"metrics_config": metrics_config});
    match call_database_api(
        Method::PATCH,
        api_endpoint,
        auth_token,
        database_name.clone(),
        Some(data),
    )
    .await?
    {
        Parsed(database) => {
            console_data!("Updating database!\n\n{database}");
        }
        Unparseable(response_text) => {
            console_data!("Updating database!");
            if !response_text.is_empty() {
                console_data!("\n\n{response_text}");
            }
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

pub async fn list_databases(
    api_endpoint: String,
    valkey_hostname: String,
    auth_token: String,
) -> Result<(), CliError> {
    let response = call_database_list_api(api_endpoint, auth_token).await?;
    let database_name = match response {
        Parsed(ListDatabasesResponse {
            databases: databases_list,
        }) => {
            if databases_list.is_empty() {
                console_data!("No databases found");
                None
            } else {
                console_data!("Databases:");
                databases_list.iter().for_each(|database| {
                    console_data!("\n{database}");
                });
                if databases_list.len() == 1 {
                    Some(databases_list[0].name.clone())
                } else {
                    Some("<DATABASE NAME>".to_string())
                }
            }
        }
        Unparseable(response_text) => {
            console_data!("Listing databases:\n\n{response_text}");
            Some("<DATABASE NAME>".to_string())
        }
    };
    if let Some(database_name) = database_name {
        print_valkey_cli_sample(valkey_hostname, &database_name);
    }
    Ok(())
}
