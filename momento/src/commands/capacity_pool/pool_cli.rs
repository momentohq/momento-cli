use super::utils::{
    call_pool_http_api, ExplicitProvisioning, ExplicitProvisioningUpdate, Response,
};
use crate::{error::CliError, utils::console::console_data};

use http::Method;
use serde_json;

pub async fn create_pool(
    endpoint: String,
    auth_token: String,
    name: String,
    instance_type: String,
    shard_count: u32,
    replicas_per_shard: u32,
    zones: Vec<String>,
) -> Result<(), CliError> {
    let data = serde_json::json!({
        "provisioning": {
            "explicit": ExplicitProvisioning {
                instance_type,
                shard_count,
                replicas_per_shard,
                zones,
            }
        }
    });
    match call_pool_http_api(Method::POST, endpoint, auth_token, name, Some(data)).await? {
        Response::Parsed(pool) => {
            console_data!(
                "Creating pool! Name: {}, Status: {}, Provisioning: {}",
                pool.name,
                pool.status,
                serde_json::to_string_pretty(&pool.provisioning.explicit)?,
            );
        }
        Response::Unparseable(response_text) => {
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
    match call_pool_http_api(Method::GET, endpoint, auth_token, name, None).await? {
        Response::Parsed(pool) => {
            console_data!("Pool status for {}: {}", pool.name, pool.status);
        }
        Response::Unparseable(response_text) => {
            console_data!("{response_text}");
        }
    };
    Ok(())
}

pub async fn update_pool(
    endpoint: String,
    auth_token: String,
    name: String,
    instance_type: Option<String>,
    shard_count: Option<u32>,
    replicas_per_shard: Option<u32>,
    zones: Option<Vec<String>>,
) -> Result<(), CliError> {
    let data = serde_json::json!({
        "provisioning": {
            "explicit": ExplicitProvisioningUpdate {
                instance_type,
                shard_count,
                replicas_per_shard,
                zones,
            }
        }
    });
    match call_pool_http_api(Method::PATCH, endpoint, auth_token, name, Some(data)).await? {
        Response::Parsed(pool) => {
            console_data!(
                "Updating pool! Name: {}, Status: {}, Provisioning: {}",
                pool.name,
                pool.status,
                serde_json::to_string_pretty(&pool.provisioning.explicit)?,
            );
        }
        Response::Unparseable(response_text) => {
            console_data!("Updating pool! {response_text}");
        }
    };
    Ok(())
}
