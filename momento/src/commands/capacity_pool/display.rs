use super::utils::{CapacityPoolProvisioning, CapacityPoolResponse};

use std::fmt;

impl fmt::Display for CapacityPoolProvisioning {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CapacityPoolProvisioning::Explicit {
                instance_type,
                shard_count,
                replicas_per_shard,
                zones,
            } => write!(
                f,
                "- Mode: explicit\n\
                 - EC2 Instance Type: {instance_type}\n\
                 - Shard Count: {shard_count}\n\
                 - Replication: {replicas_per_shard} replicas per shard\n\
                 - Availability Zones: {}",
                zones.join(", ")
            ),
            CapacityPoolProvisioning::Managed {
                capacity,
                replication,
                zones,
            } => {
                let capacity_string = format!("{}..{} GiB", capacity.min_gib, capacity.max_gib);
                let replication_string = format!(
                    "{}..{} replicas per shard",
                    replication.min_replicas_per_shard, replication.max_replicas_per_shard
                );
                write!(
                    f,
                    "- Mode: managed\n\
                     - Capacity: {capacity_string}\n\
                     - Replication: {replication_string}\n\
                     - Availability Zones: {}",
                    zones.join(", ")
                )
            }
        }
    }
}

impl fmt::Display for CapacityPoolResponse {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Name: {}\n", self.name)?;
        write!(f, "Status: {}\n", self.status)?;
        write!(f, "Provisioning:\n{}", self.provisioning)?;
        if !self.diagnostics.is_empty() {
            write!(
                f,
                "\nDiagnostics: {}",
                serde_json::to_string_pretty(&self.diagnostics)
                    .unwrap_or_else(|_| format!("{:#?}", self.diagnostics)),
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
