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

#[derive(Deserialize)]
pub struct PoolError {
    pub detail: Option<String>,
    pub message: Option<String>,
}
