use super::utils::{
    call_pool_api, call_pool_delete_api, call_pool_list_api, CapacityPoolProvisioning,
    CapacityPoolProvisioningUpdate,
};
use crate::commands::capacity_pool::utils::ListCapacityPoolsResponse;
use crate::commands::utils::MomentoHttpResponse::{Parsed, Unparseable};
use crate::{error::CliError, utils::console::console_data};

use http::Method;
use serde_json;

pub async fn create_pool(
    momento_endpoint: &str,
    valkey_endpoint: &str,
    auth_token: String,
    name: String,
    provisioning: CapacityPoolProvisioning,
) -> Result<(), CliError> {
    let data = serde_json::json!({"provisioning": provisioning});
    match call_pool_api(Method::POST, momento_endpoint, auth_token, name, Some(data)).await? {
        Parsed(pool) => {
            console_data!("Creating capacity pool!\n\n{pool}");
        }
        Unparseable(response_text) => {
            console_data!("Creating capacity pool!");
            if !response_text.is_empty() {
                console_data!("\n\n{response_text}");
            }
        }
    };
    console_data!("\nAfter you create a database, you can use the Valkey CLI to interact with your capacity pool:\n");
    console_data!("{valkey_endpoint}");
    Ok(())
}

pub async fn get_status(endpoint: &str, auth_token: String, name: String) -> Result<(), CliError> {
    match call_pool_api(Method::GET, endpoint, auth_token, name, None).await? {
        Parsed(pool) => {
            console_data!("{}", pool.status);
        }
        Unparseable(response_text) => {
            console_data!("{response_text}");
        }
    };
    Ok(())
}

pub async fn describe_pool(
    endpoint: &str,
    auth_token: String,
    name: String,
) -> Result<(), CliError> {
    match call_pool_api(Method::GET, endpoint, auth_token, name, None).await? {
        Parsed(pool) => {
            console_data!("Your capacity pool:\n\n{pool}");
        }
        Unparseable(response_text) => {
            console_data!("Your capacity pool:\n\n{response_text}");
        }
    };
    Ok(())
}

pub async fn update_pool(
    endpoint: &str,
    auth_token: String,
    name: String,
    provisioning_update: CapacityPoolProvisioningUpdate,
) -> Result<(), CliError> {
    let data = serde_json::json!({"provisioning": provisioning_update});
    match call_pool_api(Method::PATCH, endpoint, auth_token, name, Some(data)).await? {
        Parsed(mut pool) => {
            pool.hide_lagging_target(provisioning_update);
            console_data!("Updating capacity pool!\n\n{pool}");
        }
        Unparseable(response_text) => {
            console_data!("Updating capacity pool!");
            if !response_text.is_empty() {
                console_data!("\n\n{response_text}");
            }
        }
    };
    Ok(())
}

pub async fn delete_pool(endpoint: &str, auth_token: String, name: String) -> Result<(), CliError> {
    let response_text = call_pool_delete_api(endpoint, auth_token, name.clone()).await?;
    console_data!("Deleting capacity pool {name}!");
    if !response_text.is_empty() {
        console_data!("\n\n{response_text}");
    }
    Ok(())
}

pub async fn list_pools(endpoint: &str, auth_token: String) -> Result<(), CliError> {
    let response = call_pool_list_api(endpoint, auth_token).await?;
    match response {
        Parsed(ListCapacityPoolsResponse {
            capacity_pools: pools_list,
        }) => {
            if pools_list.is_empty() {
                console_data!("No capacity pools found");
            } else {
                console_data!("Capacity pools:");
                for pool in pools_list.iter() {
                    console_data!("\n{pool}");
                }
            }
        }
        Unparseable(response_text) => {
            console_data!("Listing your capacity pools:\n\n{response_text}");
        }
    };
    Ok(())
}
