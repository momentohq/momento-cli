use super::utils::{ExplicitProvisioning, ExplicitProvisioningUpdate, PoolError, PoolResponse};
use crate::{error::CliError, utils::console::console_data};

use http::Method;
use log::{info, warn};
use reqwest;
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
    let request_url = format!("{endpoint}/capacity_pool/{name}");
    let data = serde_json::json!({
        "provisioning": {
            "explicit": ExplicitProvisioning {
                instance_type,
                shard_count,
                replicas_per_shard,
                zones,
            }
        }
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
        match serde_json::from_str::<PoolResponse>(response_text.as_str()) {
            Ok(pool) => {
                info!("Response sent back by {name} pool creation:\n{pool:#?}");
                console_data!(
                    "Creating pool! Name: {}, Status: {}, Provisioning: {:#?}",
                    pool.name,
                    pool.status,
                    pool.provisioning.explicit,
                );
            }
            Err(err) => {
                warn!("Can't parse response from {name} pool creation:\n{response_text}\n{err}");
                console_data!("Creating pool! {response_text}");
            }
        };
        Ok(())
    } else {
        let error_text = response.text().await?;
        Err(CliError::new(if error_text.is_empty() {
            format!("{status}")
        } else {
            let error_message = match serde_json::from_str::<PoolError>(error_text.as_str()) {
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

pub async fn get_status(
    endpoint: String,
    auth_token: String,
    name: String,
) -> Result<(), CliError> {
    let request_url = format!("{endpoint}/capacity_pool/{name}");

    let req_client = reqwest::Client::builder().build()?;
    let response = req_client
        .request(Method::GET, &request_url)
        .header("authorization", &auth_token)
        .send()
        .await?;
    let status = response.status();
    if status.is_success() {
        let response_text = response.text().await?;
        match serde_json::from_str::<PoolResponse>(response_text.as_str()) {
            Ok(pool) => {
                info!("Response sent back by {name} pool describe:\n{pool:#?}");
                console_data!("Pool status for {}: {}", pool.name, pool.status);
            }
            Err(err) => {
                warn!("Can't parse response from {name} pool describe:\n{response_text}\n{err}");
                console_data!("{response_text}");
            }
        };
        Ok(())
    } else {
        let error_text = response.text().await?;
        Err(CliError::new(if error_text.is_empty() {
            format!("{status}")
        } else {
            let error_message = match serde_json::from_str::<PoolError>(error_text.as_str()) {
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

pub async fn update_pool(
    endpoint: String,
    auth_token: String,
    name: String,
    instance_type: Option<String>,
    shard_count: Option<u32>,
    replicas_per_shard: Option<u32>,
    zones: Option<Vec<String>>,
) -> Result<(), CliError> {
    let request_url = format!("{endpoint}/capacity_pool/{name}");
    let data = serde_json::json!({
        "provisioning": {
            "explicit": ExplicitProvisioningUpdate {
                instance_type,
                shard_count,
                replicas_per_shard,
                zones,
            }
        }
    })
    .to_string();

    let req_client = reqwest::Client::builder().build()?;
    let response = req_client
        .request(Method::PATCH, &request_url)
        .body(data)
        .header("authorization", &auth_token)
        .header("content-type", "application/json")
        .send()
        .await?;
    let status = response.status();
    if status.is_success() {
        let response_text = response.text().await?;
        match serde_json::from_str::<PoolResponse>(response_text.as_str()) {
            Ok(pool) => {
                info!("Response sent back by {name} pool update:\n{pool:#?}");
                console_data!(
                    "Updating pool! Name: {}, Status: {}, Provisioning: {:#?}",
                    pool.name,
                    pool.status,
                    pool.provisioning.explicit,
                );
            }
            Err(err) => {
                warn!("Can't parse response from {name} pool update:\n{response_text}\n{err}");
                console_data!("Updating pool! {response_text}");
            }
        };
        Ok(())
    } else {
        let error_text = response.text().await?;
        Err(CliError::new(if error_text.is_empty() {
            format!("{status}")
        } else {
            let error_message = match serde_json::from_str::<PoolError>(error_text.as_str()) {
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
