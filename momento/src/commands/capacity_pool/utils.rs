use crate::error::CliError;

use http::Method;
use log::{info, warn};
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json;

#[derive(Deserialize, Serialize, Debug)]
pub struct ExplicitProvisioning {
    pub instance_type: String,
    pub shard_count: u32,
    pub replicas_per_shard: u32,
    pub zones: Vec<String>,
}

#[derive(Serialize)]
pub struct ExplicitProvisioningUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shard_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replicas_per_shard: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zones: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct Provisioning {
    pub explicit: ExplicitProvisioning,
}

#[derive(Debug, Deserialize)]
pub struct PoolResponse {
    pub name: String,
    pub status: String,
    pub provisioning: Provisioning,
}

#[derive(Deserialize)]
struct PoolError {
    pub detail: Option<String>,
    pub message: Option<String>,
}

pub enum Response {
    Parsed(PoolResponse),
    Unparseable(String),
}

pub async fn call_pool_http_api(
    method: Method,
    endpoint: String,
    auth_token: String,
    name: String,
    data: Option<serde_json::Value>,
) -> Result<Response, CliError> {
    let request_url = format!("{endpoint}/capacity_pool/{name}");

    let req_client = reqwest::Client::builder().build()?;
    let response = req_client
        .request(method.clone(), &request_url)
        .header("authorization", &auth_token)
        .header("content-type", "application/json")
        .body(data.unwrap_or_default().to_string())
        .send()
        .await?;
    let status = response.status();

    if status.is_success() {
        let response_text = response.text().await?;
        match serde_json::from_str::<PoolResponse>(response_text.as_str()) {
            Ok(response) => {
                info!("Response sent back from {method} /capacity_pool/{name}:\n{response:#?}");
                Ok(Response::Parsed(response))
            }
            Err(err) => {
                warn!(
                    "Can't parse response from {method} /capacity_pool/{name}: \
                    \n{response_text}\n{err}"
                );
                Ok(Response::Unparseable(response_text))
            }
        }
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
