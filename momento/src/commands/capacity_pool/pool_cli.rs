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
    endpoint: String,
    auth_token: String,
    name: String,
    provisioning: CapacityPoolProvisioning,
) -> Result<(), CliError> {
    let data = serde_json::json!({"provisioning": provisioning});
    match call_pool_api(Method::POST, endpoint, auth_token, name, Some(data)).await? {
        Parsed(pool) => {
            console_data!(
                "Creating pool! Name: {}, Status: {}, Provisioning: {}",
                pool.name,
                pool.status,
                serde_json::to_string_pretty(&pool.provisioning)?,
            );
        }
        Unparseable(response_text) => {
            console_data!("Creating pool! {response_text}");
        }
    };
    Ok(())
}

pub async fn get_status(
    endpoint: String,
    auth_token: String,
    name: String,
) -> Result<(), CliError> {
    match call_pool_api(Method::GET, endpoint, auth_token, name, None).await? {
        Parsed(pool) => {
            console_data!("Pool status for {}: {}", pool.name, pool.status);
        }
        Unparseable(response_text) => {
            console_data!("{response_text}");
        }
    };
    Ok(())
}

pub async fn update_pool(
    endpoint: String,
    auth_token: String,
    name: String,
    provisioning_update: CapacityPoolProvisioningUpdate,
) -> Result<(), CliError> {
    let data = serde_json::json!({"provisioning": provisioning_update});
    match call_pool_api(Method::PATCH, endpoint, auth_token, name, Some(data)).await? {
        Parsed(pool) => {
            console_data!(
                "Updating pool! Name: {}, Status: {}, Provisioning: {}",
                pool.name,
                pool.status,
                serde_json::to_string_pretty(&pool.provisioning)?,
            );
        }
        Unparseable(response_text) => {
            console_data!("Updating pool! {response_text}");
        }
    };
    Ok(())
}

pub async fn delete_pool(
    endpoint: String,
    auth_token: String,
    name: String,
) -> Result<(), CliError> {
    let response_text = call_pool_delete_api(endpoint, auth_token, name).await?;
    console_data!("Deleting pool! {response_text}");
    Ok(())
}

pub async fn list_pools(endpoint: String, auth_token: String) -> Result<(), CliError> {
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
                    console_data!(
                        "\nName: {}, Status: {}, Provisioning: {}, Diagnostics: {}",
                        pool.name,
                        pool.status,
                        serde_json::to_string_pretty(&pool.provisioning)?,
                        serde_json::to_string_pretty(&pool.diagnostics)?,
                    );
                }
            }
        }
        Unparseable(response_text) => {
            console_data!("Listing capacity pools:\n{response_text}");
        }
    };
    Ok(())
}
