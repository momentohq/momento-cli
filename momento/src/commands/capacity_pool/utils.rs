use crate::commands::utils::{
    call_momento_http_api, call_momento_http_api_raw, MomentoHttpData, MomentoHttpResponse,
};
use crate::error::CliError;
use momento_cli_opts::{Bounds, CapacityPoolProvisioningMode};

use http::Method;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CapacityBounds {
    pub min_gib: u32,
    pub max_gib: u32,
}

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
pub struct FlexProvisioning {
    pub capacity: CapacityBounds,
    pub replication: ReplicationBounds,
    pub zones: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
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
#[derive(Debug, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CapacityPoolDiagnostics(pub Vec<CapacityPoolDiagnosticEntry>);

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
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

impl CapacityPoolResponse {
    /// If provisioning bounds change, then target changes as needed on *next* reconciler tick.
    /// Until then, hide the target for clarity.
    pub fn hide_lagging_target(&mut self, provisioning_update: CapacityPoolProvisioningUpdate) {
        if let CapacityPoolProvisioningUpdate::Flex {
            capacity,
            replication,
            ..
        } = provisioning_update
        {
            if capacity.is_some() {
                self.allocation.target_capacity_gib = None;
            }
            if replication.is_some() {
                self.allocation.target_replicas_per_shard = None;
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
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
    let provisioning = match ((instance_type.clone(), shard_count), capacity_gib) {
        ((Some(instance_type), Some(shard_count)), None) => {
            let replicas_per_shard = pinned(replicas_per_shard)?;
            CapacityPoolProvisioning::Cluster {
                instance_type,
                shard_count,
                replicas_per_shard,
                zones,
            }
        }
        ((None, None), Some(capacity)) => CapacityPoolProvisioning::Flex(FlexProvisioning {
            capacity: CapacityBounds::from(capacity),
            replication: ReplicationBounds::from(replicas_per_shard),
            zones,
        }),
        _ => {
            let shared_args = "--replicas-per-shard\n--zones";
            let help_text = format!(
                "For cluster mode, specify all of:\n--instance-type\n--shard-count\n{shared_args}\n\n\
                 For flex mode, specify all of:\n--capacity-gib\n{shared_args}"
            );
            return Err(CliError::new(format!(
                "{}\n\n{help_text}",
                match ((instance_type, shard_count), capacity_gib) {
                    ((Some(_), None), None) => "Missing --shard-count.",
                    ((None, Some(_)), None) => "Missing --instance-type.",
                    ((None, None), None) => "Missing argument(s).",
                    ((Some(_), _), Some(_)) | ((_, Some(_)), Some(_)) => "Conflicting arguments.",
                    ((Some(_), Some(_)), None) | ((None, None), Some(_)) => {
                        // This should never happen; valid combination that should have been identified earlier.
                        "Sorry, something went wrong!"
                    }
                }
            )));
        }
    };
    Ok(provisioning)
}

pub fn determine_provisioning_update(
    mode: Option<CapacityPoolProvisioningMode>,
    instance_type: Option<String>,
    shard_count: Option<u32>,
    replicas_per_shard: Option<Bounds>,
    capacity_gib: Option<Bounds>,
    zones: Vec<String>,
) -> Result<CapacityPoolProvisioningUpdate, CliError> {
    let has_cluster_field = instance_type.is_some() || shard_count.is_some();
    let has_flex_field = capacity_gib.is_some();
    let has_ambiguous_field = replicas_per_shard.is_some() || !zones.is_empty();
    let update = match (
        has_cluster_field,
        has_flex_field,
        has_ambiguous_field,
        mode.clone(),
    ) {
        (true, false, _, None | Some(CapacityPoolProvisioningMode::Cluster))
        | (_, false, true, Some(CapacityPoolProvisioningMode::Cluster)) => {
            let replicas_per_shard = replicas_per_shard.map(pinned).transpose()?;
            CapacityPoolProvisioningUpdate::Cluster {
                instance_type,
                shard_count,
                replicas_per_shard,
                zones,
            }
        }
        (false, true, _, None | Some(CapacityPoolProvisioningMode::Flex))
        | (false, _, true, Some(CapacityPoolProvisioningMode::Flex)) => {
            CapacityPoolProvisioningUpdate::Flex {
                capacity: capacity_gib.map(CapacityBounds::from),
                replication: replicas_per_shard.map(ReplicationBounds::from),
                zones,
            }
        }
        _ => {
            let shared_args = "--replicas-per-shard\n--zones";
            let help_text = format!(
                "For cluster mode, specify one or more of:\n--instance-type\n--shard-count\n\n\
                 For flex mode, specify one or more of:\n--capacity-gib\n\n\
                 With a mode specified, you can also specify one or more of:\n{shared_args}"
            );
            return Err(CliError::new(format!(
                "{}\n\n{help_text}",
                match (has_cluster_field, has_flex_field, has_ambiguous_field, mode,) {
                    (false, false, true, None) => "Missing --mode.",
                    (false, false, false, _) => "Missing field(s) to update.",
                    (true, true, _, _)
                    | (true, _, _, Some(CapacityPoolProvisioningMode::Flex))
                    | (_, true, _, Some(CapacityPoolProvisioningMode::Cluster)) =>
                        "Conflicting arguments.",
                    (true, false, _, None | Some(CapacityPoolProvisioningMode::Cluster))
                    | (false, true, _, None | Some(CapacityPoolProvisioningMode::Flex))
                    | (false, false, true, Some(_)) => {
                        // This should never happen; valid combination that should have been identified earlier.
                        "Sorry, something went wrong!"
                    }
                },
            )));
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

    fn bounds(min: u32, max: u32) -> Bounds {
        Bounds { min, max }
    }

    fn strings<const N: usize>(values: [&str; N]) -> Vec<String> {
        values.into_iter().map(String::from).collect()
    }

    fn assert_reason(err: &CliError, want: &str) {
        assert_reason_for("", err, want);
    }

    fn assert_reason_for(test_case: &str, err: &CliError, want: &str) {
        // first line only; don't compare to full, standardized help text
        let reason = err.msg.lines().next().unwrap_or_default();
        assert!(
            reason.contains(want),
            "{}expected first line to contain {want:?}; full message:\n{}",
            if !test_case.is_empty() {
                format!("{test_case}: ")
            } else {
                "".to_string()
            },
            err.msg
        );
    }

    // ========== ========== ===========
    // determine_provisioning (create)
    // ========== ========== ===========

    #[test]
    fn test_determine_provisioning_in_cluster_mode() {
        let provisioning = determine_provisioning(
            Some("r7g.xlarge".to_string()),
            Some(3),
            bounds(2, 2),
            None,
            strings(["use1-az1", "use1-az2"]),
        )
        .expect("cluster-mode args should provision");

        let CapacityPoolProvisioning::Cluster {
            instance_type,
            shard_count,
            replicas_per_shard,
            zones,
        } = provisioning
        else {
            panic!("expected cluster-mode provisioning, got {provisioning:?}");
        };
        assert_eq!("r7g.xlarge", instance_type);
        assert_eq!(3, shard_count);
        assert_eq!(2, replicas_per_shard);
        assert_eq!(strings(["use1-az1", "use1-az2"]), zones);
    }

    #[test]
    fn test_determine_provisioning_in_flex_mode() {
        let provisioning = determine_provisioning(
            None,
            None,
            bounds(1, 3),
            Some(bounds(100, 500)),
            strings(["use1-az1"]),
        )
        .expect("flex-mode args should provision");

        let CapacityPoolProvisioning::Flex(flex) = provisioning else {
            panic!("expected flex provisioning, got {provisioning:?}");
        };
        assert_eq!(100, flex.capacity.min_gib);
        assert_eq!(500, flex.capacity.max_gib);
        assert_eq!(1, flex.replication.min_replicas_per_shard);
        assert_eq!(3, flex.replication.max_replicas_per_shard);
        assert_eq!(strings(["use1-az1"]), flex.zones);
    }

    #[test]
    fn test_determine_provisioning_requires_all_fields_in_cluster_mode() {
        let err = determine_provisioning(
            Some("r7g.xlarge".to_string()),
            None,
            bounds(2, 2),
            None,
            strings(["use1-az1"]),
        )
        .expect_err("--instance-type without --shard-count should be rejected");
        assert_reason(&err, "Missing --shard-count");

        let err = determine_provisioning(None, Some(3), bounds(2, 2), None, strings(["use1-az1"]))
            .expect_err("--shard-count without --instance-type should be rejected");
        assert_reason(&err, "Missing --instance-type");
    }

    #[test]
    fn test_determine_provisioning_requires_mode() {
        let err = determine_provisioning(None, None, bounds(2, 2), None, strings(["use1-az1"]))
            .expect_err("no mode args at all should be rejected");

        assert_reason(&err, "Missing arg");
        assert!(
            err.msg.contains("cluster") && err.msg.contains("flex"),
            "error should name both modes, got: {}",
            err.msg
        );
    }

    #[test]
    fn test_determine_provisioning_rejects_conflicting_fields() {
        for (instance_type, shard_count) in [
            (Some("r7g.xlarge".to_string()), None),
            (None, Some(3)),
            (Some("r7g.xlarge".to_string()), Some(3)),
        ] {
            let case = format!("instance_type={instance_type:?} shard_count={shard_count:?}");
            let err = determine_provisioning(
                instance_type,
                shard_count,
                bounds(2, 2),
                Some(bounds(100, 500)),
                strings(["use1-az1"]),
            )
            .expect_err(&format!("{case} with --capacity-gib should be rejected"));

            assert_reason_for(&case, &err, "Conflicting arg");
            assert!(
                err.msg.contains("mode"),
                "error should mention field(s) not matching mode, got: {}",
                err.msg
            );
        }
    }

    #[test]
    fn test_determine_provisioning_requires_pinned_replication_in_cluster_mode() {
        let err = determine_provisioning(
            Some("r7g.xlarge".to_string()),
            Some(3),
            bounds(1, 3),
            None,
            strings(["use1-az1"]),
        )
        .expect_err("a replication range should be rejected in cluster mode");

        assert!(
            err.msg.contains("--replicas-per-shard") && err.msg.contains("mode"),
            "error should ask for a pinned --replicas-per-shard in this mode, got: {}",
            err.msg
        );
    }

    // ========== ========== ===========
    // determine_provisioning_update, with --mode specified
    // ========== ========== ===========

    #[test]
    fn test_determine_provisioning_update_in_cluster_mode_with_one_field() {
        let update = determine_provisioning_update(
            Some(CapacityPoolProvisioningMode::Cluster),
            None,
            Some(5),
            None,
            None,
            vec![],
        )
        .expect("a single cluster-mode field should be a valid update");

        let CapacityPoolProvisioningUpdate::Cluster {
            instance_type,
            shard_count,
            replicas_per_shard,
            zones,
        } = update
        else {
            panic!("expected a cluster-mode update, got {update:?}");
        };
        assert_eq!(None, instance_type);
        assert_eq!(Some(5), shard_count);
        assert_eq!(None, replicas_per_shard);
        assert_eq!(Vec::<String>::new(), zones);
    }

    #[test]
    fn test_determine_provisioning_update_in_cluster_mode_with_all_fields() {
        let update = determine_provisioning_update(
            Some(CapacityPoolProvisioningMode::Cluster),
            Some("r7g.xlarge".to_string()),
            Some(3),
            Some(bounds(2, 2)),
            None,
            strings(["use1-az1", "use1-az2"]),
        )
        .expect("all cluster-mode fields should be a valid update");

        let CapacityPoolProvisioningUpdate::Cluster {
            instance_type,
            shard_count,
            replicas_per_shard,
            zones,
        } = update
        else {
            panic!("expected a cluster-mode update, got {update:?}");
        };
        assert_eq!(Some("r7g.xlarge".to_string()), instance_type);
        assert_eq!(Some(3), shard_count);
        assert_eq!(Some(2), replicas_per_shard);
        assert_eq!(strings(["use1-az1", "use1-az2"]), zones);
    }

    #[test]
    fn test_determine_provisioning_update_in_flex_mode_with_one_field() {
        let update = determine_provisioning_update(
            Some(CapacityPoolProvisioningMode::Flex),
            None,
            None,
            None,
            Some(bounds(100, 500)),
            vec![],
        )
        .expect("a single flex-mode field should be a valid update");

        let CapacityPoolProvisioningUpdate::Flex {
            capacity,
            replication,
            zones,
        } = update
        else {
            panic!("expected a flex-mode update, got {update:?}");
        };
        let capacity = capacity.expect("capacity should be updated");
        assert_eq!(100, capacity.min_gib);
        assert_eq!(500, capacity.max_gib);
        assert!(replication.is_none(), "replication should be unchanged");
        assert_eq!(Vec::<String>::new(), zones);
    }

    #[test]
    fn test_determine_provisioning_update_in_flex_mode_with_all_fields() {
        let update = determine_provisioning_update(
            Some(CapacityPoolProvisioningMode::Flex),
            None,
            None,
            Some(bounds(1, 3)),
            Some(bounds(100, 500)),
            strings(["use1-az1"]),
        )
        .expect("all flex-mode fields should be a valid update");

        let CapacityPoolProvisioningUpdate::Flex {
            capacity,
            replication,
            zones,
        } = update
        else {
            panic!("expected a flex-mode update, got {update:?}");
        };
        let capacity = capacity.expect("capacity should be updated");
        assert_eq!(100, capacity.min_gib);
        assert_eq!(500, capacity.max_gib);
        let replication = replication.expect("replication should be updated");
        assert_eq!(1, replication.min_replicas_per_shard);
        assert_eq!(3, replication.max_replicas_per_shard);
        assert_eq!(strings(["use1-az1"]), zones);
    }

    #[test]
    fn test_determine_provisioning_update_with_only_zones() {
        let zones = strings(["use1-az1", "use1-az2"]);

        let update = determine_provisioning_update(
            Some(CapacityPoolProvisioningMode::Cluster),
            None,
            None,
            None,
            None,
            zones.clone(),
        )
        .expect("a zones-only update should be valid with cluster mode specified");
        let CapacityPoolProvisioningUpdate::Cluster { zones: got, .. } = update else {
            panic!("expected a cluster-mode update, got {update:?}");
        };
        assert_eq!(zones, got);

        let update = determine_provisioning_update(
            Some(CapacityPoolProvisioningMode::Flex),
            None,
            None,
            None,
            None,
            zones.clone(),
        )
        .expect("a zones-only update should be valid with flex mode specified");
        let CapacityPoolProvisioningUpdate::Flex { zones: got, .. } = update else {
            panic!("expected a flex-mode update, got {update:?}");
        };
        assert_eq!(zones, got);
    }

    #[test]
    fn test_determine_provisioning_update_with_only_replication() {
        let update = determine_provisioning_update(
            Some(CapacityPoolProvisioningMode::Cluster),
            None,
            None,
            Some(bounds(2, 2)),
            None,
            vec![],
        )
        .expect("a replicas-only update should be valid with cluster mode specified");
        let CapacityPoolProvisioningUpdate::Cluster {
            replicas_per_shard, ..
        } = update
        else {
            panic!("expected a cluster-mode update, got {update:?}");
        };
        assert_eq!(Some(2), replicas_per_shard);

        let update = determine_provisioning_update(
            Some(CapacityPoolProvisioningMode::Flex),
            None,
            None,
            Some(bounds(1, 3)),
            None,
            vec![],
        )
        .expect("a replicas-only update should be valid with flex mode specified");
        let CapacityPoolProvisioningUpdate::Flex { replication, .. } = update else {
            panic!("expected a flex-mode update, got {update:?}");
        };
        let replication = replication.expect("replication should be updated");
        assert_eq!(1, replication.min_replicas_per_shard);
        assert_eq!(3, replication.max_replicas_per_shard);
    }

    #[test]
    fn test_determine_provisioning_update_with_no_fields() {
        for mode in [
            CapacityPoolProvisioningMode::Cluster,
            CapacityPoolProvisioningMode::Flex,
        ] {
            let case = format!("{mode:?} mode");
            let err = determine_provisioning_update(Some(mode), None, None, None, None, vec![])
                .expect_err(&format!(
                    "{case}: an update with no fields should be rejected"
                ));

            assert_reason_for(&case, &err, "Missing field");
        }
    }

    #[test]
    fn test_determine_provisioning_update_rejects_flex_fields_in_cluster_mode() {
        let err = determine_provisioning_update(
            Some(CapacityPoolProvisioningMode::Cluster),
            None,
            None,
            None,
            Some(bounds(100, 500)),
            vec![],
        )
        .expect_err("--capacity-gib should be rejected in cluster mode");

        assert_reason(&err, "Conflicting arg");
        assert!(
            err.msg.contains("--capacity-gib") && err.msg.contains("mode"),
            "error should mention --capacity-gib doesn't match the mode, got: {}",
            err.msg
        );
    }

    #[test]
    fn test_determine_provisioning_update_rejects_cluster_fields_in_flex_mode() {
        for (instance_type, shard_count) in [
            (Some("r7g.xlarge".to_string()), None),
            (None, Some(3)),
            (Some("r7g.xlarge".to_string()), Some(3)),
        ] {
            let case = format!("instance_type={instance_type:?} shard_count={shard_count:?}");
            let err = determine_provisioning_update(
                Some(CapacityPoolProvisioningMode::Flex),
                instance_type,
                shard_count,
                None,
                None,
                vec![],
            )
            .expect_err(&format!("{case} should be rejected in flex mode"));

            assert_reason_for(&case, &err, "Conflicting arg");
            assert!(
                err.msg.contains("mode"),
                "error should mention field(s) not matching mode, got: {}",
                err.msg
            );
        }
    }

    #[test]
    fn test_determine_provisioning_update_rejects_conflicting_fields_in_either_mode() {
        for mode in [
            CapacityPoolProvisioningMode::Cluster,
            CapacityPoolProvisioningMode::Flex,
        ] {
            let case = format!("{mode:?} mode");
            let err = determine_provisioning_update(
                Some(mode),
                None,
                Some(3),
                None,
                Some(bounds(100, 500)),
                vec![],
            )
            .expect_err(&format!(
                "{case}: cluster-mode and flex-mode fields together should be rejected"
            ));

            assert_reason_for(&case, &err, "Conflicting arg");
            assert!(
                err.msg.contains("mode"),
                "error should mention field(s) not matching mode, got: {}",
                err.msg
            );
        }
    }

    #[test]
    fn test_determine_provisioning_update_requires_pinned_replication_in_cluster_mode() {
        let err = determine_provisioning_update(
            Some(CapacityPoolProvisioningMode::Cluster),
            None,
            None,
            Some(bounds(1, 3)),
            None,
            vec![],
        )
        .expect_err("a replication range should be rejected in cluster mode");

        assert!(
            err.msg.contains("--replicas-per-shard") && err.msg.contains("mode"),
            "error should ask for a pinned --replicas-per-shard in this mode, got: {}",
            err.msg
        );
    }

    // ========== ========== ===========
    // determine_provisioning_update, with --mode inferred
    // ========== ========== ===========

    #[test]
    fn test_determine_provisioning_update_infers_cluster_mode() {
        let update = determine_provisioning_update(
            None,
            Some("r7g.xlarge".to_string()),
            None,
            Some(bounds(2, 2)),
            None,
            strings(["use1-az1"]),
        )
        .expect("cluster-mode fields should imply cluster mode");

        let CapacityPoolProvisioningUpdate::Cluster {
            instance_type,
            shard_count,
            replicas_per_shard,
            zones,
        } = update
        else {
            panic!("expected a cluster-mode update, got {update:?}");
        };
        assert_eq!(Some("r7g.xlarge".to_string()), instance_type);
        assert_eq!(None, shard_count);
        assert_eq!(Some(2), replicas_per_shard);
        assert_eq!(strings(["use1-az1"]), zones);
    }

    #[test]
    fn test_determine_provisioning_update_infers_flex_mode() {
        let update = determine_provisioning_update(
            None,
            None,
            None,
            Some(bounds(1, 3)),
            Some(bounds(100, 500)),
            strings(["use1-az1"]),
        )
        .expect("flex-mode fields should imply flex mode");

        let CapacityPoolProvisioningUpdate::Flex {
            capacity,
            replication,
            zones,
        } = update
        else {
            panic!("expected a flex-mode update, got {update:?}");
        };
        let capacity = capacity.expect("capacity should be updated");
        assert_eq!(100, capacity.min_gib);
        assert_eq!(500, capacity.max_gib);
        let replication = replication.expect("replication should be updated");
        assert_eq!(1, replication.min_replicas_per_shard);
        assert_eq!(3, replication.max_replicas_per_shard);
        assert_eq!(strings(["use1-az1"]), zones);
    }

    #[test]
    fn test_determine_provisioning_update_requires_mode_for_ambiguous_fields() {
        for (replicas_per_shard, zones) in [
            (Some(bounds(2, 2)), vec![]),
            (None, strings(["use1-az1"])),
            (Some(bounds(1, 3)), strings(["use1-az1"])),
        ] {
            let case = format!("replicas_per_shard={replicas_per_shard:?} zones={zones:?}");
            let err =
                determine_provisioning_update(None, None, None, replicas_per_shard, None, zones)
                    .expect_err(&format!("{case} without --mode should be rejected"));

            assert_reason_for(&case, &err, "Missing --mode");
        }
    }

    #[test]
    fn test_determine_provisioning_update_with_no_fields_no_mode() {
        let err = determine_provisioning_update(None, None, None, None, None, vec![])
            .expect_err("an update with no fields and no mode should be rejected");

        assert_reason(&err, "Missing field");
    }

    #[test]
    fn test_determine_provisioning_update_rejects_conflicting_fields_with_no_mode() {
        let err = determine_provisioning_update(
            None,
            None,
            Some(3),
            None,
            Some(bounds(100, 500)),
            vec![],
        )
        .expect_err("cluster-mode and flex-mode fields together should be rejected");

        assert_reason(&err, "Conflicting arg");
    }

    #[test]
    fn test_determine_provisioning_update_requires_pinned_replication_in_inferred_cluster_mode() {
        let err =
            determine_provisioning_update(None, None, Some(3), Some(bounds(1, 3)), None, vec![])
                .expect_err("a replication range should be rejected once cluster mode is inferred");

        assert!(
            err.msg.contains("--replicas-per-shard") && err.msg.contains("mode"),
            "error should ask for a pinned --replicas-per-shard in this mode, got: {}",
            err.msg
        );
    }

    // ========== ========== ===========
    // Serialization
    // ========== ========== ===========

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
            zones: strings(["use1-az1"]),
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
            zones: strings(["use1-az2", "use1-az3"]),
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
            zones: strings(["use1-az1"]),
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
            zones: strings(["use1-az2", "use1-az3"]),
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
    fn test_serialize_provisioning_update_in_flex_mode_with_no_fields() {
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
    fn test_serialize_provisioning_update_in_cluster_mode_with_no_fields() {
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

    // ========== ========== ===========
    // Deserialization
    // ========== ========== ===========

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
    fn test_deserialize_capacity_pool_in_flex_mode_with_all_fields() {
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
    fn test_deserialize_capacity_pool_in_cluster_mode_with_all_fields() {
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
        assert_eq!(&strings(["use1-az1"]), zones);
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
        assert_eq!(&strings(["use1-az1"]), zones);
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
