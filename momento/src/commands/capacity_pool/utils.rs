use crate::commands::utils::{
    call_momento_http_api, call_momento_http_api_raw, MomentoHttpData, MomentoHttpResponse,
};
use crate::error::CliError;
use momento_cli_opts::{Bounds, CapacityPoolProvisioningMode};

use http::Method;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct CapacityBounds {
    pub min_gb: u32,
    pub max_gb: u32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ReplicationBounds {
    pub min_replicas_per_shard: u32,
    pub max_replicas_per_shard: u32,
}

impl From<Bounds> for CapacityBounds {
    fn from(bounds: Bounds) -> Self {
        Self {
            min_gb: bounds.min,
            max_gb: bounds.max,
        }
    }
}

impl From<Bounds> for ReplicationBounds {
    fn from(bounds: Bounds) -> Self {
        Self {
            min_replicas_per_shard: bounds.min,
            max_replicas_per_shard: bounds.max,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CapacityPoolProvisioning {
    Explicit {
        instance_type: String,
        shard_count: u32,
        replicas_per_shard: u32,
        zones: Vec<String>,
    },
    Managed {
        capacity: CapacityBounds,
        replication: ReplicationBounds,
        zones: Vec<String>,
    },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CapacityPoolProvisioningUpdate {
    Explicit {
        #[serde(skip_serializing_if = "Option::is_none")]
        instance_type: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        shard_count: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        replicas_per_shard: Option<u32>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        zones: Vec<String>,
    },
    Managed {
        #[serde(skip_serializing_if = "Option::is_none")]
        capacity: Option<CapacityBounds>,
        #[serde(skip_serializing_if = "Option::is_none")]
        replication: Option<ReplicationBounds>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        zones: Vec<String>,
    },
}

#[derive(Debug, Deserialize)]
pub struct CapacityPoolResponse {
    pub name: String,
    pub status: String,
    pub provisioning: CapacityPoolProvisioning,
    #[serde(default)]
    pub diagnostics: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct ListCapacityPoolsResponse {
    pub capacity_pools: Vec<CapacityPoolResponse>,
}

/// The single, pinned `--replicas-per-shard` that's required by explicit provisioning.
fn pinned(bounds: Bounds) -> Result<u32, CliError> {
    (bounds.min == bounds.max)
        .then_some(bounds.min)
        .ok_or_else(|| {
            CliError::new(
                "explicit pools take a single --replicas-per-shard (e.g. 2); \
                 ranges are for managed pools",
            )
        })
}

pub fn determine_provisioning(
    instance_type: Option<String>,
    shard_count: Option<u32>,
    replicas_per_shard: Bounds,
    capacity_gb: Option<Bounds>,
    zones: Vec<String>,
) -> Result<CapacityPoolProvisioning, CliError> {
    let provisioning = match (instance_type, shard_count, capacity_gb) {
        (Some(instance_type), Some(shard_count), None) => {
            let replicas_per_shard = pinned(replicas_per_shard)?;
            CapacityPoolProvisioning::Explicit {
                instance_type,
                shard_count,
                replicas_per_shard,
                zones,
            }
        }
        (None, None, Some(capacity)) => CapacityPoolProvisioning::Managed {
            capacity: CapacityBounds::from(capacity),
            replication: ReplicationBounds::from(replicas_per_shard),
            zones,
        },
        _ => {
            return Err(CliError::new(
                "pass either --instance-type with --shard-count (explicit) \
                 or --capacity-gb (managed)",
            ));
        }
    };
    Ok(provisioning)
}

pub fn determine_provisioning_update(
    mode: CapacityPoolProvisioningMode,
    instance_type: Option<String>,
    shard_count: Option<u32>,
    replicas_per_shard: Option<Bounds>,
    capacity_gb: Option<Bounds>,
    zones: Vec<String>,
) -> Result<CapacityPoolProvisioningUpdate, CliError> {
    let update = match mode {
        CapacityPoolProvisioningMode::Explicit => {
            if capacity_gb.is_some() {
                return Err(CliError::new(
                    "--capacity-gb is a managed-mode field; pass --mode managed",
                ));
            }
            let replicas_per_shard = replicas_per_shard.map(pinned).transpose()?;
            CapacityPoolProvisioningUpdate::Explicit {
                instance_type,
                shard_count,
                replicas_per_shard,
                zones,
            }
        }
        CapacityPoolProvisioningMode::Managed => {
            if instance_type.is_some() || shard_count.is_some() {
                return Err(CliError::new(
                    "--instance-type and --shard-count are explicit-mode fields; \
                     pass --mode explicit",
                ));
            }
            CapacityPoolProvisioningUpdate::Managed {
                capacity: capacity_gb.map(CapacityBounds::from),
                replication: replicas_per_shard.map(ReplicationBounds::from),
                zones,
            }
        }
    };
    Ok(update)
}

fn build_request_url(endpoint: String, pool_name: Option<String>) -> String {
    match pool_name {
        None => format!("{endpoint}/capacity_pool"),
        Some(name) => format!("{endpoint}/capacity_pool/{name}"),
    }
}

pub async fn call_pool_api(
    method: Method,
    endpoint: String,
    auth_token: String,
    pool_name: String,
    data: Option<serde_json::Value>,
) -> Result<MomentoHttpResponse<CapacityPoolResponse>, CliError> {
    call_momento_http_api(
        method,
        build_request_url(endpoint, Some(pool_name)),
        auth_token,
        None,
        data.map(MomentoHttpData::Json),
    )
    .await
}

pub async fn call_pool_delete_api(
    endpoint: String,
    auth_token: String,
    pool_name: String,
) -> Result<String, CliError> {
    call_momento_http_api_raw(
        Method::DELETE,
        build_request_url(endpoint, Some(pool_name)),
        auth_token,
        None,
        None,
    )
    .await
}

pub async fn call_pool_list_api(
    endpoint: String,
    auth_token: String,
) -> Result<MomentoHttpResponse<ListCapacityPoolsResponse>, CliError> {
    call_momento_http_api(
        Method::GET,
        build_request_url(endpoint, None),
        auth_token,
        None,
        None,
    )
    .await
}
