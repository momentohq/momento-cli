use super::utils::{call_pool_api, CapacityPoolProvisioning, CapacityPoolProvisioningUpdate};
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
