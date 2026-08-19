use crate::commands::utils::{
    call_momento_http_api, call_momento_http_api_raw, MomentoHttpData, MomentoHttpResponse,
};
use crate::error::CliError;
use momento_cli_opts::{Bounds, CapacityPoolProvisioningMode};

use http::Method;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct CapacityBounds {
    pub min_gib: u32,
    pub max_gib: u32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ReplicationBounds {
    pub min_replicas_per_shard: u32,
    pub max_replicas_per_shard: u32,
}

impl From<Bounds> for CapacityBounds {
    fn from(bounds: Bounds) -> Self {
        Self {
            min_gib: bounds.min,
            max_gib: bounds.max,
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
pub struct FlexProvisioning {
    pub capacity: CapacityBounds,
    pub replication: ReplicationBounds,
    pub zones: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum CapacityPoolProvisioning {
    #[serde(rename = "explicit")]
    Cluster {
        instance_type: String,
        shard_count: u32,
        replicas_per_shard: u32,
        zones: Vec<String>,
    },
    #[serde(rename = "managed")]
    Flex(FlexProvisioning),
}

#[derive(Debug, Serialize)]
pub enum CapacityPoolProvisioningUpdate {
    #[serde(rename = "explicit")]
    Cluster {
        #[serde(skip_serializing_if = "Option::is_none")]
        instance_type: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        shard_count: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        replicas_per_shard: Option<u32>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        zones: Vec<String>,
    },
    #[serde(rename = "managed")]
    Flex {
        #[serde(skip_serializing_if = "Option::is_none")]
        capacity: Option<CapacityBounds>,
        #[serde(skip_serializing_if = "Option::is_none")]
        replication: Option<ReplicationBounds>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        zones: Vec<String>,
    },
}

/// A single diagnostic, which the API sends as a one-entry object keyed by kind:
/// `{"insufficient_capacity": {"state": "active", ...}}`. The fields vary by kind,
/// so we keep them as raw JSON rather than modelling every variant.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(from = "serde_json::Value")]
pub enum CapacityPoolDiagnosticEntry {
    Parsed {
        kind: String,
        fields: serde_json::Map<String, serde_json::Value>,
    },
    Unparseable(serde_json::Value),
}

impl From<serde_json::Value> for CapacityPoolDiagnosticEntry {
    fn from(entry: serde_json::Value) -> Self {
        let (kind, fields) = match entry {
            serde_json::Value::Object(entry) if entry.len() == 1 => {
                entry.into_iter().next().expect("length checked above")
            }
            other => return Self::Unparseable(other),
        };
        match fields {
            serde_json::Value::Object(fields) => Self::Parsed { kind, fields },
            other => Self::Unparseable(serde_json::Value::Object(
                [(kind, other)].into_iter().collect(),
            )),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct CapacityPoolDiagnostics(pub Vec<CapacityPoolDiagnosticEntry>);

#[derive(Debug, Deserialize)]
pub struct FlexAllocation {
    /// The capacity the pool demonstrably provided in its last settled state.
    pub current_capacity_gib: Option<u32>,
    /// The replication the pool demonstrably provided in its last settled state.
    pub current_replicas_per_shard: Option<u32>,
    /// The capacity this pool is converging to; equal to current_capacity_gib except while a scale is in flight.
    pub target_capacity_gib: Option<u32>,
    /// The replication the pool is converging to; equal to current_replicas_per_shard except while a scale is in flight.
    pub target_replicas_per_shard: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct CapacityPoolResponse {
    pub name: String,
    pub status: String,
    pub provisioning: CapacityPoolProvisioning,
    pub diagnostics: Option<CapacityPoolDiagnostics>,
    #[serde(flatten)]
    /// Flex-/managed-mode pools only
    pub allocation: FlexAllocation,
    #[serde(flatten)]
    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct ListCapacityPoolsResponse {
    pub capacity_pools: Vec<CapacityPoolResponse>,
}

/// The single, pinned `--replicas-per-shard` that's required by cluster-/explicit-mode provisioning.
fn pinned(bounds: Bounds) -> Result<u32, CliError> {
    (bounds.min == bounds.max)
        .then_some(bounds.min)
        .ok_or_else(|| {
            CliError::new(
                "cluster-mode pools take a single --replicas-per-shard (e.g. 2); \
                 ranges are for flex-mode pools",
            )
        })
}

pub fn determine_provisioning(
    instance_type: Option<String>,
    shard_count: Option<u32>,
    replicas_per_shard: Bounds,
    capacity_gib: Option<Bounds>,
    zones: Vec<String>,
) -> Result<CapacityPoolProvisioning, CliError> {
    let provisioning = match (instance_type, shard_count, capacity_gib) {
        (Some(instance_type), Some(shard_count), None) => {
            let replicas_per_shard = pinned(replicas_per_shard)?;
            CapacityPoolProvisioning::Cluster {
                instance_type,
                shard_count,
                replicas_per_shard,
                zones,
            }
        }
        (None, None, Some(capacity)) => CapacityPoolProvisioning::Flex(FlexProvisioning {
            capacity: CapacityBounds::from(capacity),
            replication: ReplicationBounds::from(replicas_per_shard),
            zones,
        }),
        _ => {
            return Err(CliError::new(
                "pass either --instance-type with --shard-count (cluster mode) \
                 or --capacity-gib (flex mode)",
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
    capacity_gib: Option<Bounds>,
    zones: Vec<String>,
) -> Result<CapacityPoolProvisioningUpdate, CliError> {
    let update = match mode {
        CapacityPoolProvisioningMode::Cluster => {
            if capacity_gib.is_some() {
                return Err(CliError::new(
                    "--capacity-gib is a flex-mode field; pass --mode flex",
                ));
            }
            let replicas_per_shard = replicas_per_shard.map(pinned).transpose()?;
            CapacityPoolProvisioningUpdate::Cluster {
                instance_type,
                shard_count,
                replicas_per_shard,
                zones,
            }
        }
        CapacityPoolProvisioningMode::Flex => {
            if instance_type.is_some() || shard_count.is_some() {
                return Err(CliError::new(
                    "--instance-type and --shard-count are cluster-mode fields; \
                     pass --mode cluster",
                ));
            }
            CapacityPoolProvisioningUpdate::Flex {
                capacity: capacity_gib.map(CapacityBounds::from),
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

#[cfg(test)]
pub mod test_utils {
    pub fn field_map<const N: usize>(
        pairs: [(&str, serde_json::Value); N],
    ) -> serde_json::Map<String, serde_json::Value> {
        pairs
            .into_iter()
            .map(|(name, value)| (name.to_string(), value))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::test_utils::*;
    use super::*;

    use serde_json::json;

    fn parse_diagnostic(json: &str) -> CapacityPoolDiagnosticEntry {
        serde_json::from_str(json).expect("should parse diagnostic")
    }

    fn parse_pool(json: &str) -> CapacityPoolResponse {
        serde_json::from_str(json).expect("should parse a capacity pool")
    }

    #[test]
    fn test_serialize_provisioning_in_flex_mode() {
        let provisioning = CapacityPoolProvisioning::Flex(FlexProvisioning {
            capacity: CapacityBounds {
                min_gib: 32,
                max_gib: 128,
            },
            replication: ReplicationBounds {
                min_replicas_per_shard: 1,
                max_replicas_per_shard: 2,
            },
            zones: vec!["use1-az1".to_string()],
        });

        assert_eq!(
            json!({
                "managed": {
                    "capacity": {"min_gib": 32, "max_gib": 128},
                    "replication": {
                        "min_replicas_per_shard": 1,
                        "max_replicas_per_shard": 2
                    },
                    "zones": ["use1-az1"]
                }
            }),
            serde_json::to_value(provisioning).expect("provisioning should serialize")
        )
    }

    #[test]
    fn test_serialize_provisioning_in_cluster_mode() {
        let provisioning = CapacityPoolProvisioning::Cluster {
            instance_type: "r7g.xlarge".to_string(),
            shard_count: 3,
            replicas_per_shard: 1,
            zones: vec!["use1-az2".to_string(), "use1-az3".to_string()],
        };

        assert_eq!(
            json!({
                "explicit": {
                    "instance_type": "r7g.xlarge",
                    "shard_count": 3,
                    "replicas_per_shard": 1,
                    "zones": ["use1-az2", "use1-az3"]
                }
            }),
            serde_json::to_value(provisioning).expect("provisioning should serialize")
        );
    }

    #[test]
    fn test_serialize_provisioning_update_in_flex_mode_with_all_fields() {
        let update = CapacityPoolProvisioningUpdate::Flex {
            capacity: Some(CapacityBounds {
                min_gib: 32,
                max_gib: 128,
            }),
            replication: Some(ReplicationBounds {
                min_replicas_per_shard: 1,
                max_replicas_per_shard: 2,
            }),
            zones: vec!["use1-az1".to_string()],
        };

        assert_eq!(
            json!({
                "managed": {
                    "capacity": {"min_gib": 32, "max_gib": 128},
                    "replication": {
                        "min_replicas_per_shard": 1,
                        "max_replicas_per_shard": 2
                    },
                    "zones": ["use1-az1"]
                }
            }),
            serde_json::to_value(update).expect("provisioning update should serialize")
        )
    }

    #[test]
    fn test_serialize_provisioning_update_in_cluster_mode_with_all_fields() {
        let update = CapacityPoolProvisioningUpdate::Cluster {
            instance_type: Some("r7g.xlarge".to_string()),
            shard_count: Some(3),
            replicas_per_shard: Some(1),
            zones: vec!["use1-az2".to_string(), "use1-az3".to_string()],
        };

        assert_eq!(
            json!({
                "explicit": {
                    "instance_type": "r7g.xlarge",
                    "shard_count": 3,
                    "replicas_per_shard": 1,
                    "zones": ["use1-az2", "use1-az3"]
                }
            }),
            serde_json::to_value(update).expect("provisioning update should serialize")
        );
    }

    #[test]
    fn test_serialize_provisioning_update_in_flex_mode_with_no_updates() {
        let update = CapacityPoolProvisioningUpdate::Flex {
            capacity: None,
            replication: None,
            // Must have at least 1 availability zone, so [] is treated as No Update
            zones: vec![],
        };

        assert_eq!(
            json!({ "managed": {} }),
            serde_json::to_value(update).expect("provisioning update should serialize")
        );
    }

    #[test]
    fn test_serialize_provisioning_update_in_cluster_mode_with_no_updates() {
        let update = CapacityPoolProvisioningUpdate::Cluster {
            instance_type: None,
            shard_count: None,
            replicas_per_shard: None,
            // Must have at least 1 availability zone, so [] is treated as No Update
            zones: vec![],
        };

        assert_eq!(
            json!({ "explicit": {} }),
            serde_json::to_value(update).expect("provisioning update should serialize")
        );
    }

    #[test]
    fn test_deserialize_diagnostic_by_its_kind() {
        let diagnostic = parse_diagnostic(
            r#"{
                "insufficient_capacity": {
                    "state": "active",
                    "message": "Insufficient r7g.xlarge capacity in use1-az1.",
                    "instance_type": "r7g.xlarge",
                    "availability_zones": ["use1-az1"],
                    "first_observed_epoch_seconds": 1719360000,
                    "last_observed_epoch_seconds": 1719363600
                }
            }"#,
        );

        let CapacityPoolDiagnosticEntry::Parsed { kind, fields } = diagnostic else {
            panic!("expected a recognized diagnostic");
        };
        assert_eq!("insufficient_capacity", kind);
        assert!(!fields.is_empty());
    }

    #[test]
    fn test_deserialize_diagnostic_with_various_field_types() {
        let diagnostic = parse_diagnostic(
            r#"{
                "something_something": {
                    "state": "active",
                    "zones": ["use1-az1"],
                    "observed": 1719360000
                }
            }"#,
        );

        let CapacityPoolDiagnosticEntry::Parsed { fields, .. } = diagnostic else {
            panic!("expected a recognized diagnostic");
        };
        assert_eq!(Some(&json!("active")), fields.get("state"));
        assert_eq!(Some(&json!(["use1-az1"])), fields.get("zones"));
        assert_eq!(Some(&json!(1719360000)), fields.get("observed"));
    }

    #[test]
    fn test_deserialize_diagnostic_fields_in_api_order() {
        let diagnostic = parse_diagnostic(
            r#"{
                "insufficient_capacity": {
                    "state": "active",
                    "instance_type": "r7g.xlarge",
                    "availability_zones": ["use1-az1"],
                    "first_observed_epoch_seconds": 1719360000,
                    "message": "hello world",
                    "last_observed_epoch_seconds": 1719363600
                }
            }"#,
        );

        let CapacityPoolDiagnosticEntry::Parsed { fields, .. } = diagnostic else {
            panic!("expected a recognized diagnostic");
        };
        assert_eq!(
            vec![
                "state",
                "instance_type",
                "availability_zones",
                "first_observed_epoch_seconds",
                "message",
                "last_observed_epoch_seconds",
            ],
            fields.keys().collect::<Vec<&String>>()
        );
    }

    #[test]
    fn test_deserialize_diagnostic_as_raw_json_when_unrecognized() {
        for (json, want) in [
            (r#"{}"#, json!({})),
            (r#"{"a": {}, "b": {}}"#, json!({"a": {}, "b": {}})),
            (r#"{"answer": 42}"#, json!({"answer": 42})),
            (r#""a""#, json!("a")),
        ] {
            let diagnostic = parse_diagnostic(json);
            assert_eq!(
                CapacityPoolDiagnosticEntry::Unparseable(want),
                diagnostic,
                "{json} should survive parsing as raw json, got {diagnostic:?}"
            );
        }
    }

    #[test]
    fn test_deserialize_diagnostics_with_both_recognized_and_unrecognized() {
        let diagnostics = serde_json::from_str::<CapacityPoolDiagnostics>(
            r#"[{"answer": 42}, {"stuck": {"state": "active"}}]"#,
        )
        .expect("should parse all diagnostics")
        .0;

        assert_eq!(
            vec![
                CapacityPoolDiagnosticEntry::Unparseable(json!({"answer": 42})),
                CapacityPoolDiagnosticEntry::Parsed {
                    kind: "stuck".to_string(),
                    fields: field_map([("state", json!("active"))]),
                }
            ],
            diagnostics
        );
    }

    #[test]
    fn test_deserialize_capacity_pool_with_all_fields_in_flex_mode() {
        let pool = parse_pool(
            r#"{
                "name": "hello world",
                "status": "creating",
                "provisioning": {
                    "managed": {
                        "capacity": {"min_gib": 32, "max_gib": 128},
                        "replication": {
                            "min_replicas_per_shard": 1,
                            "max_replicas_per_shard": 2
                        },
                        "zones": ["use1-az1", "use1-az2"]
                    }
                },
                "diagnostics": [{"stuck": {"state": "resolved"}}],
                "current_capacity_gib": 40,
                "current_replicas_per_shard": 2,
                "abc": {"X": "x", "Y": "y", "Z": "z"},
                "hello": "world",
                "answer": 42
            }"#,
        );

        assert_eq!("hello world", pool.name);
        assert_eq!("creating", pool.status);
        let CapacityPoolProvisioning::Flex(provisioning) = &pool.provisioning else {
            panic!("expected flex provisioning, got {:?}", pool.provisioning);
        };
        assert_eq!(32, provisioning.capacity.min_gib);
        assert_eq!(128, provisioning.capacity.max_gib);
        assert_eq!(1, provisioning.replication.min_replicas_per_shard);
        assert_eq!(2, provisioning.replication.max_replicas_per_shard);
        assert_eq!(vec!["use1-az1", "use1-az2"], provisioning.zones);
        assert_eq!(Some(40), pool.allocation.current_capacity_gib);
        assert_eq!(Some(2), pool.allocation.current_replicas_per_shard);

        assert_eq!(
            vec![CapacityPoolDiagnosticEntry::Parsed {
                kind: "stuck".to_string(),
                fields: field_map([("state", json!("resolved"))]),
            }],
            pool.diagnostics.expect("should parse diagnostics").0
        );

        assert_eq!(
            field_map([
                ("abc", json!({"X": "x", "Y": "y", "Z": "z"})),
                ("hello", json!("world")),
                ("answer", json!(42)),
            ]),
            pool.extra_fields
        );
        // Map equality ignores order, so check the order the API sent separately.
        assert_eq!(
            vec!["abc", "hello", "answer"],
            pool.extra_fields.keys().collect::<Vec<&String>>()
        );
    }

    #[test]
    fn test_deserialize_capacity_pool_with_all_fields_in_cluster_mode() {
        let pool = parse_pool(
            r#"{
                "name": "hello world",
                "status": "creating",
                "provisioning": {
                    "explicit": {
                        "instance_type": "r7g.xlarge",
                        "shard_count": 3,
                        "replicas_per_shard": 1,
                        "zones": ["use1-az3", "use1-az4", "use1-az5"]
                    }
                },
                "diagnostics": [{"stuck": {"state": "resolved"}}],
                "abc": {"X": "x", "Y": "y", "Z": "z"},
                "hello": "world",
                "answer": 42
            }"#,
        );

        assert_eq!("hello world", pool.name);
        assert_eq!("creating", pool.status);
        let CapacityPoolProvisioning::Cluster {
            instance_type,
            shard_count,
            replicas_per_shard,
            zones,
        } = &pool.provisioning
        else {
            panic!(
                "expected cluster-mode provisioning, got {:?}",
                pool.provisioning
            );
        };
        assert_eq!("r7g.xlarge", instance_type);
        assert_eq!(3, *shard_count);
        assert_eq!(1, *replicas_per_shard);
        assert_eq!(vec!["use1-az3", "use1-az4", "use1-az5"], *zones);
        assert_eq!(None, pool.allocation.current_capacity_gib);
        assert_eq!(None, pool.allocation.current_replicas_per_shard);

        assert_eq!(
            vec![CapacityPoolDiagnosticEntry::Parsed {
                kind: "stuck".to_string(),
                fields: field_map([("state", json!("resolved"))]),
            }],
            pool.diagnostics.expect("should parse diagnostics").0
        );

        assert_eq!(
            field_map([
                ("abc", json!({"X": "x", "Y": "y", "Z": "z"})),
                ("hello", json!("world")),
                ("answer", json!(42)),
            ]),
            pool.extra_fields
        );
        // Map equality ignores order, so check the order the API sent separately.
        assert_eq!(
            vec!["abc", "hello", "answer"],
            pool.extra_fields.keys().collect::<Vec<&String>>()
        );
    }

    #[test]
    fn test_deserialize_capacity_pool_with_empty_diagnostics() {
        let pool = parse_pool(
            r#"{
                "name": "hello world",
                "status": "creating",
                "provisioning": {
                    "explicit": {
                        "instance_type": "r7g.xlarge",
                        "shard_count": 3,
                        "replicas_per_shard": 1,
                        "zones": ["use1-az1"]
                    }
                },
                "diagnostics": [],
                "abc": {"X": "x", "Y": "y", "Z": "z"},
                "hello": "world",
                "answer": 42
            }"#,
        );

        let CapacityPoolProvisioning::Cluster {
            instance_type,
            shard_count,
            replicas_per_shard,
            zones,
        } = &pool.provisioning
        else {
            panic!(
                "expected cluster-mode provisioning, got {:?}",
                pool.provisioning
            );
        };
        assert_eq!("r7g.xlarge", instance_type);
        assert_eq!(&3, shard_count);
        assert_eq!(&1, replicas_per_shard);
        assert_eq!(&vec!["use1-az1".to_string()], zones);
        assert_eq!(None, pool.allocation.current_capacity_gib);
        assert_eq!(None, pool.allocation.current_replicas_per_shard);

        assert_eq!(
            field_map([
                ("abc", json!({"X": "x", "Y": "y", "Z": "z"})),
                ("hello", json!("world")),
                ("answer", json!(42)),
            ]),
            pool.extra_fields
        );

        // Properly includes *empty* diagnostics
        // (different from the API excluding the field entirely):
        assert_eq!(
            Vec::<CapacityPoolDiagnosticEntry>::new(),
            pool.diagnostics.expect("should parse diagnostics").0,
        );
    }

    #[test]
    fn test_deserialize_capacity_pool_with_no_diagnostics() {
        let pool = parse_pool(
            r#"{
                "name": "hello world",
                "status": "creating",
                "provisioning": {
                    "explicit": {
                        "instance_type": "r7g.xlarge",
                        "shard_count": 3,
                        "replicas_per_shard": 1,
                        "zones": ["use1-az1"]
                    }
                },
                "abc": {"X": "x", "Y": "y", "Z": "z"},
                "hello": "world",
                "answer": 42
            }"#,
        );

        let CapacityPoolProvisioning::Cluster {
            instance_type,
            shard_count,
            replicas_per_shard,
            zones,
        } = &pool.provisioning
        else {
            panic!(
                "expected cluster-mode provisioning, got {:?}",
                pool.provisioning
            );
        };
        assert_eq!("r7g.xlarge", instance_type);
        assert_eq!(&3, shard_count);
        assert_eq!(&1, replicas_per_shard);
        assert_eq!(&vec!["use1-az1".to_string()], zones);
        assert_eq!(None, pool.allocation.current_capacity_gib);
        assert_eq!(None, pool.allocation.current_replicas_per_shard);

        assert_eq!(
            field_map([
                ("abc", json!({"X": "x", "Y": "y", "Z": "z"})),
                ("hello", json!("world")),
                ("answer", json!(42)),
            ]),
            pool.extra_fields
        );

        // Properly excludes diagnostics
        // (different from the API returning an empty field):
        assert!(
            pool.diagnostics.is_none(),
            "diagnostics should be None when excluded, got {:?}",
            pool.diagnostics
        );
    }

    #[test]
    fn test_deserialize_capacity_pool_with_no_additional_fields() {
        let pool = parse_pool(
            r#"{
                "name": "hello world",
                "status": "creating",
                "provisioning": {
                    "explicit": {
                        "instance_type": "r7g.xlarge",
                        "shard_count": 3,
                        "replicas_per_shard": 1,
                        "zones": ["use1-az1"]
                    }
                },
                "diagnostics": [{"stuck": {"state": "resolved"}}]
            }"#,
        );

        let CapacityPoolProvisioning::Cluster {
            instance_type,
            shard_count,
            replicas_per_shard,
            zones,
        } = &pool.provisioning
        else {
            panic!(
                "expected cluster-mode provisioning, got {:?}",
                pool.provisioning
            );
        };
        assert_eq!("r7g.xlarge", instance_type);
        assert_eq!(&3, shard_count);
        assert_eq!(&1, replicas_per_shard);
        assert_eq!(&vec!["use1-az1".to_string()], zones);
        assert_eq!(None, pool.allocation.current_capacity_gib);
        assert_eq!(None, pool.allocation.current_replicas_per_shard);
        assert_eq!(
            vec![CapacityPoolDiagnosticEntry::Parsed {
                kind: "stuck".to_string(),
                fields: field_map([("state", json!("resolved"))]),
            }],
            pool.diagnostics.expect("should parse diagnostics").0
        );

        // Properly excludes optional fields:
        assert!(
            pool.extra_fields.is_empty(),
            "known fields should not land in extra_fields, got {:?}",
            pool.extra_fields
        );
    }
}
