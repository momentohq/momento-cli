use super::utils::{
    CapacityPoolDiagnosticEntry, CapacityPoolDiagnostics, CapacityPoolProvisioning,
    CapacityPoolResponse, ManagedProvisioning,
};

use chrono::prelude::DateTime;
use std::fmt;

fn format_managed_provisioning(
    provisioning: &ManagedProvisioning,
    current_capacity_gib: Option<u32>,
    current_replicas_per_shard: Option<u32>,
) -> String {
    let capacity_range = format!(
        "{}..{} GiB",
        provisioning.capacity.min_gib, provisioning.capacity.max_gib
    );
    let capacity = match current_capacity_gib {
        None => capacity_range,
        Some(capacity) => format!("{capacity_range} (currently {capacity})"),
    };
    let replication_range = format!(
        "{}..{} per shard",
        provisioning.replication.min_replicas_per_shard,
        provisioning.replication.max_replicas_per_shard,
    );
    let replication = match current_replicas_per_shard {
        None => replication_range,
        Some(replicas) => format!("{replication_range} (currently {replicas})"),
    };
    format!(
        "- Capacity: {capacity}\n\
         - Replicas: {replication}\n\
         - Availability Zones: {}",
        provisioning.zones.join(", ")
    )
}

/// Fields worth reading first; everything else follows in the order the API sent it.
const LEADING_DIAGNOSTIC_FIELDS: [&str; 2] = ["state", "message"];

/// Fields (like `first_observed_epoch_seconds`) to pretty print as a date.
const EPOCH_SECONDS_SUFFIX: &str = "_epoch_seconds";

fn format_diagnostic_field_name(name: &str) -> String {
    name.strip_suffix(EPOCH_SECONDS_SUFFIX)
        .unwrap_or(name)
        .replace('_', " ")
}

fn format_diagnostic_field_value(name: &str, value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(text) => text.clone(),
        serde_json::Value::Number(number) => {
            if name.ends_with(EPOCH_SECONDS_SUFFIX) {
                match number
                    .as_i64()
                    .and_then(|seconds| DateTime::from_timestamp(seconds, 0))
                {
                    Some(datetime) => datetime.to_string(),
                    None => format!("{number} (epoch seconds)"),
                }
            } else {
                number.to_string()
            }
        }
        serde_json::Value::Array(items) => items
            .iter()
            .map(|value| format_diagnostic_field_value(name, value))
            .collect::<Vec<_>>()
            .join(", "),
        other => other.to_string(),
    }
}

impl fmt::Display for CapacityPoolDiagnosticEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (kind, fields) = match self {
            Self::Parsed { kind, fields } => (kind, fields),
            Self::Unparseable(raw) => {
                return write!(
                    f,
                    "- Unrecognized diagnostic: {}",
                    serde_json::to_string_pretty(raw).unwrap_or_else(|_| format!("{:#?}", raw))
                )
            }
        };
        write!(f, "- {kind}")?;
        for name in LEADING_DIAGNOSTIC_FIELDS {
            if let Some(value) = fields.get(name) {
                write!(
                    f,
                    "\n  {}: {}",
                    format_diagnostic_field_name(name),
                    format_diagnostic_field_value(name, value)
                )?;
            }
        }
        for (name, value) in fields {
            if !LEADING_DIAGNOSTIC_FIELDS.contains(&name.as_str()) {
                write!(
                    f,
                    "\n  {}: {}",
                    format_diagnostic_field_name(name),
                    format_diagnostic_field_value(name, value)
                )?;
            }
        }
        Ok(())
    }
}

impl fmt::Display for CapacityPoolDiagnostics {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.0.is_empty() {
            return write!(f, "(none)");
        }
        write!(
            f,
            "{}",
            self.0
                .iter()
                .map(|diagnostic| diagnostic.to_string())
                .collect::<Vec<String>>()
                .join("\n")
        )?;
        Ok(())
    }
}

impl fmt::Display for CapacityPoolResponse {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Name: {}", self.name)?;
        write!(f, "\nStatus: {}", self.status)?;
        write!(
            f,
            "\n{}",
            match &self.provisioning {
                CapacityPoolProvisioning::Explicit {
                    instance_type,
                    shard_count,
                    replicas_per_shard,
                    zones,
                } => format!(
                    "Explicit Provisioning:\n\
                     - EC2 Instance Type: {instance_type}\n\
                     - Shard Count: {shard_count}\n\
                     - Replicas: {replicas_per_shard} per shard\n\
                     - Availability Zones: {}",
                    zones.join(", "),
                ),
                CapacityPoolProvisioning::Managed(provisioning) => format!(
                    "Managed Provisioning:\n{}",
                    format_managed_provisioning(
                        provisioning,
                        self.current_capacity_gib,
                        self.current_replicas_per_shard
                    )
                ),
            }
        )?;
        if let Some(diagnostics) = &self.diagnostics {
            let string = diagnostics.to_string();
            write!(
                f,
                "\nDiagnostics:{}{}",
                if string.contains("\n") { "\n" } else { " " },
                diagnostics
            )?;
        }
        if !self.extra_fields.is_empty() {
            write!(f, "\nAdditional details:")?;
            for (field, value) in &self.extra_fields {
                write!(
                    f,
                    "\n- {field}: {}",
                    serde_json::to_string_pretty(value).unwrap_or_else(|_| format!("{:#?}", value)),
                )?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::utils::test_utils::field_map;
    use super::super::utils::{CapacityBounds, ReplicationBounds};
    use super::*;

    use serde_json::json;

    fn snapshot_settings() -> insta::Settings {
        let mut settings = insta::Settings::clone_current();
        settings.set_prepend_module_to_snapshot(false);
        settings
    }

    #[test]
    fn test_format_diagnostic_field_value_parses_timestamps() {
        assert_eq!(
            "2024-06-26 00:00:00 UTC",
            format_diagnostic_field_value("first_observed_epoch_seconds", &json!(1719360000)),
        );
        assert_eq!(
            "2024-06-26 01:00:00 UTC",
            format_diagnostic_field_value("last_observed_epoch_seconds", &json!(1719363600)),
        );
        assert_eq!(
            "9223372036854775807 (epoch seconds)",
            format_diagnostic_field_value(
                "resolved_epoch_seconds",
                &json!(serde_json::Number::from(0x7FFF_FFFF_FFFF_FFFF as u64))
            ),
        );
    }

    #[test]
    fn test_format_managed_provisioning_with_current_values() {
        let provisioning = ManagedProvisioning {
            capacity: CapacityBounds {
                min_gib: 32,
                max_gib: 128,
            },
            replication: ReplicationBounds {
                min_replicas_per_shard: 1,
                max_replicas_per_shard: 2,
            },
            zones: vec!["use1-az1".to_string()],
        };

        snapshot_settings().bind(|| {
            insta::assert_snapshot!(format_managed_provisioning(
                &provisioning,
                Some(40),
                Some(2)
            ))
        });
    }

    #[test]
    fn test_display_diagnostic_with_state_and_message_first() {
        let diagnostic = CapacityPoolDiagnosticEntry::Parsed {
            kind: "something_something".to_string(),
            fields: field_map([
                ("instance_type", json!("r7g.xlarge")),
                ("state", json!("active")),
                ("zone", json!("use1-az1")),
                ("observed", json!("today")),
                ("message", json!("Something's wrong")),
            ]),
        };

        let string = diagnostic.to_string();
        assert!(
            string.starts_with(
                "- something_something\n  \
                   state: active\n  \
                   message: Something's wrong\n"
            ),
            "should display diagnostic's state and message first, got:\n{string}"
        );
    }

    #[test]
    fn test_display_diagnostic_various_field_types_in_api_order() {
        let diagnostic = CapacityPoolDiagnosticEntry::Parsed {
            kind: "something_something".to_string(),
            fields: field_map([
                ("instance_type", json!("r7g.xlarge")),
                ("state", json!("active")),
                ("zones", json!(["use1-az1", "use1-az2", "use1-az3"])),
                ("observed", json!(1719360000)),
                ("message", json!("Something's wrong")),
            ]),
        };

        let string = diagnostic.to_string();
        assert!(
            string.ends_with(
                "\n  \
                     instance type: r7g.xlarge\n  \
                     zones: use1-az1, use1-az2, use1-az3\n  \
                     observed: 1719360000"
            ),
            "should display all field types and preserve API order, got:\n{string}"
        );
    }

    #[test]
    fn test_display_diagnostic_placeholder_when_empty_diagnostics() {
        assert_eq!("(none)", CapacityPoolDiagnostics(vec![]).to_string());
    }

    #[test]
    fn test_display_diagnostic_as_raw_json_when_unrecognized() {
        snapshot_settings().bind(|| {
            insta::assert_snapshot!(
                CapacityPoolDiagnosticEntry::Unparseable(json!({"answer": 42})).to_string()
            )
        });
    }

    #[test]
    fn test_display_diagnostics_with_both_recognized_and_unrecognized() {
        let diagnostics = CapacityPoolDiagnostics(vec![
            CapacityPoolDiagnosticEntry::Unparseable(json!({"surprise": 42})),
            CapacityPoolDiagnosticEntry::Parsed {
                kind: "stuck".to_string(),
                fields: field_map([("state", json!("active"))]),
            },
        ]);

        snapshot_settings().bind(|| insta::assert_snapshot!(diagnostics.to_string()));
    }

    #[test]
    fn test_display_capacity_pool_with_all_managed_fields() {
        let provisioning = CapacityPoolProvisioning::Managed(ManagedProvisioning {
            capacity: CapacityBounds {
                min_gib: 32,
                max_gib: 128,
            },
            replication: ReplicationBounds {
                min_replicas_per_shard: 1,
                max_replicas_per_shard: 2,
            },
            zones: vec!["use1-az1".to_string(), "use1-az2".to_string()],
        });
        let diagnostics = CapacityPoolDiagnostics(vec![
            CapacityPoolDiagnosticEntry::Parsed {
                kind: "scale_blocked_by_utilization".to_string(),
                fields: field_map([
                    ("requested_shard_count", json!(6)),
                    ("state", json!("resolved")),
                    (
                        "message",
                        json!("The requested configuration is smaller than the pool's current data; retrying until it fits."),
                    ),
                    ("requested_instance_type", json!("r7g.xlarge")),
                    ("data_approx", json!("42 GiB")),
                    ("capacity_approx", json!("32 GiB")),
                    ("first_observed_epoch_seconds", json!(1719360000)),
                    ("last_observed_epoch_seconds", json!(1719363600)),
                ]),
            },
            CapacityPoolDiagnosticEntry::Parsed {
                kind: "something_something".to_string(),
                fields: field_map([]),
            },
            CapacityPoolDiagnosticEntry::Parsed {
                kind: "something_else".to_string(),
                fields: field_map([]),
            },
        ]);
        let response = CapacityPoolResponse {
            name: "hello world".to_string(),
            provisioning,
            status: "creating".to_string(),
            diagnostics: Some(diagnostics),
            current_capacity_gib: Some(40),
            current_replicas_per_shard: Some(2),
            extra_fields: field_map([
                ("abc", json!({"X": "x", "Y": "y", "Z": "z"})),
                ("hello", json!("world")),
                ("answer", json!(42)),
            ]),
        };

        snapshot_settings().bind(|| insta::assert_snapshot!(response.to_string()));
    }

    #[test]
    fn test_display_capacity_pool_with_no_current_values_in_managed_mode() {
        let provisioning = CapacityPoolProvisioning::Managed(ManagedProvisioning {
            capacity: CapacityBounds {
                min_gib: 32,
                max_gib: 128,
            },
            replication: ReplicationBounds {
                min_replicas_per_shard: 1,
                max_replicas_per_shard: 2,
            },
            zones: vec!["use1-az1".to_string(), "use1-az2".to_string()],
        });
        let diagnostics = CapacityPoolDiagnostics(vec![
            CapacityPoolDiagnosticEntry::Parsed {
                kind: "something_something".to_string(),
                fields: field_map([
                    ("state", json!("active")),
                    ("message", json!("Something's wrong")),
                ]),
            },
            CapacityPoolDiagnosticEntry::Parsed {
                kind: "something_else".to_string(),
                fields: field_map([]),
            },
        ]);
        let response = CapacityPoolResponse {
            name: "hello world".to_string(),
            provisioning,
            status: "creating".to_string(),
            diagnostics: Some(diagnostics),
            // create-pool sends back only the requested ranges, no current/concrete values
            current_capacity_gib: None,
            current_replicas_per_shard: None,
            extra_fields: field_map([("answer", json!(42))]),
        };

        snapshot_settings().bind(|| insta::assert_snapshot!(response.to_string()));
    }

    #[test]
    fn test_display_capacity_pool_with_all_explicit_fields() {
        let provisioning = CapacityPoolProvisioning::Explicit {
            instance_type: "r7g.xlarge".to_string(),
            shard_count: 3,
            replicas_per_shard: 1,
            zones: vec![
                "use1-az3".to_string(),
                "use1-az4".to_string(),
                "use1-az5".to_string(),
            ],
        };
        let diagnostics = CapacityPoolDiagnostics(vec![
            CapacityPoolDiagnosticEntry::Parsed {
                kind: "scale_blocked_by_utilization".to_string(),
                fields: field_map([
                    ("requested_shard_count", json!(6)),
                    ("state", json!("resolved")),
                    (
                        "message",
                        json!("The requested configuration is smaller than the pool's current data; retrying until it fits."),
                    ),
                    ("requested_instance_type", json!("r7g.xlarge")),
                    ("data_approx", json!("42 GiB")),
                    ("capacity_approx", json!("32 GiB")),
                    ("first_observed_epoch_seconds", json!(1719360000)),
                    ("last_observed_epoch_seconds", json!(1719363600)),
                ]),
            },
            CapacityPoolDiagnosticEntry::Parsed {
                kind: "something_something".to_string(),
                fields: field_map([]),
            },
            CapacityPoolDiagnosticEntry::Parsed {
                kind: "something_else".to_string(),
                fields: field_map([]),
            },
        ]);
        let response = CapacityPoolResponse {
            name: "hello world".to_string(),
            provisioning,
            status: "creating".to_string(),
            diagnostics: Some(diagnostics),
            current_capacity_gib: None,
            current_replicas_per_shard: None,
            extra_fields: field_map([
                ("abc", json!({"X": "x", "Y": "y", "Z": "z"})),
                ("hello", json!("world")),
                ("answer", json!(42)),
            ]),
        };

        snapshot_settings().bind(|| insta::assert_snapshot!(response.to_string()));
    }

    #[test]
    fn test_display_capacity_pool_with_empty_diagnostics() {
        let provisioning = CapacityPoolProvisioning::Explicit {
            instance_type: "r7g.xlarge".to_string(),
            shard_count: 3,
            replicas_per_shard: 1,
            zones: vec!["usw2-az1".to_string()],
        };
        let response = CapacityPoolResponse {
            name: "hello world".to_string(),
            provisioning,
            status: "creating".to_string(),
            diagnostics: Some(CapacityPoolDiagnostics(vec![])),
            current_capacity_gib: None,
            current_replicas_per_shard: None,
            extra_fields: field_map([
                ("abc", json!({"X": "x", "Y": "y", "Z": "z"})),
                ("hello", json!("world")),
                ("answer", json!(42)),
            ]),
        };

        snapshot_settings().bind(|| insta::assert_snapshot!(response.to_string()));
    }

    #[test]
    fn test_display_capacity_pool_with_no_diagnostics() {
        let provisioning = CapacityPoolProvisioning::Explicit {
            instance_type: "r7g.xlarge".to_string(),
            shard_count: 3,
            replicas_per_shard: 1,
            zones: vec!["usw2-az1".to_string()],
        };
        let response = CapacityPoolResponse {
            name: "hello world".to_string(),
            provisioning,
            status: "creating".to_string(),
            diagnostics: None,
            current_capacity_gib: None,
            current_replicas_per_shard: None,
            extra_fields: field_map([
                ("abc", json!({"X": "x", "Y": "y", "Z": "z"})),
                ("hello", json!("world")),
                ("answer", json!(42)),
            ]),
        };

        snapshot_settings().bind(|| insta::assert_snapshot!(response.to_string()));
    }

    #[test]
    fn test_display_capacity_pool_with_no_additional_fields() {
        let provisioning = CapacityPoolProvisioning::Explicit {
            instance_type: "r7g.xlarge".to_string(),
            shard_count: 3,
            replicas_per_shard: 1,
            zones: vec!["usw2-az1".to_string()],
        };
        let diagnostics = CapacityPoolDiagnostics(vec![
            CapacityPoolDiagnosticEntry::Parsed {
                kind: "something_something".to_string(),
                fields: field_map([
                    ("state", json!("active")),
                    ("message", json!("Something's wrong")),
                ]),
            },
            CapacityPoolDiagnosticEntry::Parsed {
                kind: "something_else".to_string(),
                fields: field_map([]),
            },
        ]);
        let response = CapacityPoolResponse {
            name: "hello world".to_string(),
            provisioning,
            status: "creating".to_string(),
            diagnostics: Some(diagnostics),
            current_capacity_gib: None,
            current_replicas_per_shard: None,
            extra_fields: serde_json::Map::new(),
        };

        snapshot_settings().bind(|| insta::assert_snapshot!(response.to_string()));
    }

    #[test]
    fn test_deserialize_through_display() {
        let response: CapacityPoolResponse = serde_json::from_str(
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
                "diagnostics": [
                    {
                        "insufficient_capacity": {
                            "state": "resolved",
                            "message": "Insufficient r7g.xlarge capacity in use1-az1.",
                            "instance_type": "r7g.xlarge",
                            "availability_zones": ["use1-az1"],
                            "first_observed_epoch_seconds": 1719360000,
                            "last_observed_epoch_seconds": 1719363600
                        }
                    },
                    { "something_something": {} },
                    { "something_else": {} }
                ],
                "current_capacity_gib": 40,
                "current_replicas_per_shard": 2,
                "abc": {"X": "x", "Y": "y", "Z": "z"},
                "hello": "world",
                "answer": 42
            }"#,
        )
        .expect("should parse a capacity pool");

        snapshot_settings().bind(|| insta::assert_snapshot!(response.to_string()));
    }
}
