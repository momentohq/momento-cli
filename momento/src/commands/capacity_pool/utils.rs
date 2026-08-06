use crate::commands::utils::{call_momento_http_api, MomentoHttpData, MomentoHttpResponse};

use crate::error::CliError;
use http::Method;
use serde::{Deserialize, Serialize};

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

pub async fn call_pool_api(
    method: Method,
    endpoint: String,
    auth_token: String,
    pool_name: String,
    data: Option<serde_json::Value>,
) -> Result<MomentoHttpResponse<PoolResponse>, CliError> {
    call_momento_http_api(
        method,
        format!("{endpoint}/capacity_pool/{pool_name}"),
        auth_token,
        None,
        data.map(MomentoHttpData::Json),
    )
    .await
}
