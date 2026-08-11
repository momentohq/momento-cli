use super::utils::{CapacityPoolProvisioning, CapacityPoolResponse, ManagedProvisioning};

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
        "{}..{} replicas per shard",
        provisioning.replication.min_replicas_per_shard,
        provisioning.replication.max_replicas_per_shard,
    );
    let replication = match current_replicas_per_shard {
        None => replication_range,
        Some(replicas) => format!("{replication_range} (currently {replicas})"),
    };
    format!(
        "- Capacity: {capacity}\n\
         - Replication: {replication}\n\
         - Availability Zones: {}",
        provisioning.zones.join(", ")
    )
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
                     - Replication: {replicas_per_shard} replicas per shard\n\
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
