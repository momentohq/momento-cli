use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use aws_config::SdkConfig;
use aws_sdk_dynamodb::types::{TimeToLiveDescription, TimeToLiveStatus};
use governor::DefaultDirectRateLimiter;
use indicatif::{ProgressBar, ProgressStyle};
use phf::{phf_map, Map};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

use crate::commands::cloud_linter::metrics::{
    AppendMetrics, Metric, MetricTarget, ResourceWithMetrics,
};
use crate::commands::cloud_linter::resource::{DynamoDbResource, Resource, ResourceType};
use crate::commands::cloud_linter::utils::rate_limit;
use crate::error::CliError;

const DDB_TABLE_METRICS: Map<&'static str, &'static [&'static str]> = phf_map! {
        "Sum" => &[
            "ConsumedReadCapacityUnits",
            "ConsumedWriteCapacityUnits",
            "ProvisionedReadCapacityUnits",
            "ProvisionedWriteCapacityUnits",
            "ReadThrottleEvents",
            "WriteThrottleEvents",
            "TimeToLiveDeletedItemCount",
            "TransactionConflict",
            "ConditionalCheckFailedRequests",
        ],
        "Average" => &[
            "ConsumedReadCapacityUnits",
            "ConsumedWriteCapacityUnits",
            "ProvisionedReadCapacityUnits",
            "ProvisionedWriteCapacityUnits",
        ],
        "Maximum" => &[
            "ConsumedReadCapacityUnits",
            "ConsumedWriteCapacityUnits",
            "ProvisionedReadCapacityUnits",
            "ProvisionedWriteCapacityUnits",
            "ReadThrottleEvents",
            "WriteThrottleEvents",
        ],
};

const DDB_GSI_METRICS: Map<&'static str, &'static [&'static str]> = phf_map! {
    "Sum" => &[
            "ConsumedReadCapacityUnits",
            "ConsumedWriteCapacityUnits",
            "ProvisionedReadCapacityUnits",
            "ProvisionedWriteCapacityUnits",
            "ReadThrottleEvents",
            "WriteThrottleEvents",
        ],
    "Average" => &[
            "ConsumedReadCapacityUnits",
            "ConsumedWriteCapacityUnits",
            "ProvisionedReadCapacityUnits",
            "ProvisionedWriteCapacityUnits",
        ],
    "Maximum" => &[
            "ConsumedReadCapacityUnits",
            "ConsumedWriteCapacityUnits",
            "ProvisionedReadCapacityUnits",
            "ProvisionedWriteCapacityUnits",
            "ReadThrottleEvents",
            "WriteThrottleEvents",
        ],
};

#[derive(Serialize, Clone, Debug)]
pub(crate) struct DynamoDbMetadata {
    #[serde(rename = "avgItemSizeBytes")]
    avg_item_size_bytes: i64,
    #[serde(rename = "billingMode")]
    billing_mode: Option<String>,
    #[serde(rename = "gsiCount")]
    gsi_count: i64,
    #[serde(rename = "itemCount")]
    item_count: i64,
    #[serde(rename = "ttlEnabled")]
    ttl_enabled: bool,
    #[serde(rename = "isGlobalTable")]
    is_global_table: bool,
    #[serde(rename = "deleteProtectionEnabled")]
    delete_protection_enabled: bool,
    #[serde(rename = "lsiCount")]
    lsi_count: i64,
    #[serde(rename = "tableClass")]
    table_class: Option<String>,
    #[serde(rename = "tableSizeBytes")]
    table_size_bytes: i64,
    #[serde(rename = "pThroughputDecreasesDay")]
    p_throughput_decreases_day: Option<i64>,
    #[serde(rename = "pThroughputReadUnits")]
    p_throughput_read_units: Option<i64>,
    #[serde(rename = "pThroughputWriteUnits")]
    p_throughput_write_units: Option<i64>,
    gsi: Option<GsiMetadata>,
}

impl DynamoDbMetadata {
    fn clone_with_gsi(&self, gsi_metadata: GsiMetadata) -> DynamoDbMetadata {
        DynamoDbMetadata {
            gsi: Some(gsi_metadata),
            ..self.clone()
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct GsiMetadata {
    #[serde(rename = "gsiName")]
    gsi_name: String,
    #[serde(rename = "itemCount")]
    item_count: i64,
    #[serde(rename = "projectionType")]
    projection_type: Option<String>,
    #[serde(rename = "sizeBytes")]
    size_bytes: i64,
    #[serde(rename = "pThroughputDecreasesDay")]
    p_throughput_decreases_day: Option<i64>,
    #[serde(rename = "pThroughputReadUnits")]
    p_throughput_read_units: Option<i64>,
    #[serde(rename = "pThroughputWriteUnits")]
    p_throughput_write_units: Option<i64>,
}

impl ResourceWithMetrics for DynamoDbResource {
    fn create_metric_targets(&self) -> Result<Vec<MetricTarget>, CliError> {
        match self.resource_type {
            ResourceType::DynamoDbTable => Ok(vec![MetricTarget {
                prefix: "".to_string(),
                namespace: "AWS/DynamoDB".to_string(),
                expression: "".to_string(),
                dimensions: HashMap::from([("TableName".to_string(), self.id.clone())]),
                targets: DDB_TABLE_METRICS,
            }]),
            ResourceType::DynamoDbGsi => {
                let gsi_name = self
                    .metadata
                    .gsi
                    .as_ref()
                    .map(|gsi| gsi.gsi_name.clone())
                    .ok_or(CliError {
                        msg: "Global secondary index name not found".to_string(),
                    })?;
                Ok(vec![MetricTarget {
                    prefix: "".to_string(),
                    namespace: "AWS/DynamoDB".to_string(),
                    expression: "".to_string(),
                    dimensions: HashMap::from([
                        ("TableName".to_string(), self.id.clone()),
                        ("GlobalSecondaryIndexName".to_string(), gsi_name),
                    ]),
                    targets: DDB_GSI_METRICS,
                }])
            }
            _ => Err(CliError {
                msg: "Invalid resource type".to_string(),
            }),
        }
    }

    fn set_metrics(&mut self, metrics: Vec<Metric>) {
        self.metrics = metrics;
    }

    fn set_metric_period_seconds(&mut self, period: i32) {
        self.metric_period_seconds = period;
    }
}

pub(crate) async fn process_ddb_resources(
    config: &SdkConfig,
    control_plane_limiter: Arc<DefaultDirectRateLimiter>,
    metrics_limiter: Arc<DefaultDirectRateLimiter>,
    describe_ttl_limiter: Arc<DefaultDirectRateLimiter>,
    sender: Sender<Resource>,
) -> Result<(), CliError> {
    let ddb_client = aws_sdk_dynamodb::Client::new(config);
    let metrics_client = aws_sdk_cloudwatch::Client::new(config);

    let list_ddb_tables_bar = ProgressBar::new_spinner().with_message("Listing Dynamo DB tables");
    list_ddb_tables_bar.enable_steady_tick(Duration::from_millis(100));
    let table_names = list_table_names(&ddb_client, Arc::clone(&control_plane_limiter)).await?;
    list_ddb_tables_bar.finish();

    let describe_ddb_tables_bar =
        ProgressBar::new(table_names.len() as u64).with_message("Processing Dynamo DB tables");
    describe_ddb_tables_bar
        .set_style(ProgressStyle::with_template("  {msg} {bar} {eta}").expect("invalid template"));
    for table_name in table_names {
        process_table_resources(
            &ddb_client,
            &metrics_client,
            &table_name,
            Arc::clone(&control_plane_limiter),
            Arc::clone(&metrics_limiter),
            Arc::clone(&describe_ttl_limiter),
            sender.clone(),
        )
        .await?;
        describe_ddb_tables_bar.inc(1);
    }

    describe_ddb_tables_bar.finish();
    Ok(())
}

async fn list_table_names(
    ddb_client: &aws_sdk_dynamodb::Client,
    limiter: Arc<DefaultDirectRateLimiter>,
) -> Result<Vec<String>, CliError> {
    let mut table_names = Vec::new();
    let mut name_stream = ddb_client.list_tables().into_paginator().send();

    while let Some(result) = rate_limit(Arc::clone(&limiter), || name_stream.next()).await {
        match result {
            Ok(result) => {
                if let Some(names) = result.table_names {
                    table_names.extend(names);
                }
            }
            Err(err) => {
                return Err(CliError {
                    msg: format!("Failed to list Dynamo DB table names: {}", err),
                });
            }
        }
    }

    Ok(table_names)
}

async fn is_ddb_ttl_enabled(
    ddb_client: &aws_sdk_dynamodb::Client,
    resource: &DynamoDbResource,
    limiter: Arc<DefaultDirectRateLimiter>,
) -> Result<bool, CliError> {
    if resource.resource_type == ResourceType::DynamoDbGsi {
        return Ok(false);
    };
    let consumed_write_ops_index = resource.metrics.iter().position(|p| {
        p.name
            .eq_ignore_ascii_case("consumedwritecapacityunits_sum")
    });
    match consumed_write_ops_index {
        Some(index) => {
            let consumed_write_capacity = resource.metrics.get(index).expect("index should exist");
            let sum: f64 = consumed_write_capacity.values.iter().sum();
            // a basic heuristic around whether we care to check to see if a ttl exists on a ddb table. If the dynamodb table
            // has less than 10 tps average, then we don't care to check if ttl is enabled or not.
            if sum < 10.0 * 60.0 * 60.0 * 24.0 * 30.0 {
                log::debug!("skipping ttl check for table {}", resource.id);
                return Ok(false);
            }
        }
        // we did not find that metric, and therefore we assume that there are no consumed capacity units, meaning we don't care to
        // check for a ttl on the ddb table
        None => {
            return Ok(false);
        }
    }
    log::debug!("querying ttl for table {}", resource.id);
    let ttl = rate_limit(Arc::clone(&limiter), || async {
        ddb_client
            .describe_time_to_live()
            .table_name(&resource.id)
            .send()
            .await
    })
    .await?;

    let ttl_enabled = matches!(
        ttl.time_to_live_description,
        Some(TimeToLiveDescription {
            time_to_live_status: Some(TimeToLiveStatus::Enabled),
            ..
        })
    );
    Ok(ttl_enabled)
}

async fn process_table_resources(
    ddb_client: &aws_sdk_dynamodb::Client,
    metrics_client: &aws_sdk_cloudwatch::Client,
    table_name: &str,
    control_plane_limiter: Arc<DefaultDirectRateLimiter>,
    metrics_limiter: Arc<DefaultDirectRateLimiter>,
    describe_ttl_limiter: Arc<DefaultDirectRateLimiter>,
    sender: Sender<Resource>,
) -> Result<(), CliError> {
    let region = ddb_client
        .config()
        .region()
        .map(|r| r.as_ref())
        .ok_or(CliError {
            msg: "No region configured for client".to_string(),
        })?;

    let description = rate_limit(Arc::clone(&control_plane_limiter), || async {
        ddb_client
            .describe_table()
            .table_name(table_name)
            .send()
            .await
    })
    .await?;

    let table = description.table.ok_or(CliError {
        msg: "Table description not found".to_string(),
    })?;

    let item_count = table.item_count.unwrap_or_default();
    let table_size_bytes = table.table_size_bytes.unwrap_or_default();
    let avg_item_size_bytes = if item_count > 0 {
        table_size_bytes / item_count
    } else {
        0
    };

    let billing_mode = table
        .billing_mode_summary
        .and_then(|summary| summary.billing_mode)
        .map(|billing_mode| billing_mode.as_str().to_string());

    let table_class = table
        .table_class_summary
        .and_then(|summary| summary.table_class)
        .map(|class| class.as_str().to_string());

    let gsi_count = table
        .global_secondary_indexes
        .as_ref()
        .map(|gsi| gsi.len() as i64)
        .unwrap_or_default();

    let lsi_count = table
        .local_secondary_indexes
        .map(|lsi| lsi.len() as i64)
        .unwrap_or_default();

    let is_global_table = table.global_table_version.is_some();

    let (p_throughput_decreases_day, p_throughput_read_units, p_throughput_write_units) = table
        .provisioned_throughput
        .as_ref()
        .map(|p| {
            (
                p.number_of_decreases_today,
                p.read_capacity_units,
                p.write_capacity_units,
            )
        })
        .unwrap_or_default();

    let metadata = DynamoDbMetadata {
        avg_item_size_bytes,
        billing_mode,
        gsi_count,
        item_count,
        ttl_enabled: false,
        is_global_table,
        lsi_count,
        table_class,
        table_size_bytes,
        p_throughput_decreases_day,
        p_throughput_read_units,
        p_throughput_write_units,
        gsi: None,
        delete_protection_enabled: table.deletion_protection_enabled.unwrap_or_default(),
    };

    let mut resources = table
        .global_secondary_indexes
        .as_ref()
        .map(|gsis| {
            let mut instances = Vec::with_capacity(gsis.len() + 1);
            for gsi in gsis {
                let gsi_name = gsi
                    .index_name
                    .as_ref()
                    .ok_or(CliError {
                        msg: "Global secondary index name not found".to_string(),
                    })?
                    .clone();

                let gsi_item_count = gsi.item_count.ok_or(CliError {
                    msg: "Global secondary index item count not found".to_string(),
                })?;

                let gsi_size_bytes = gsi.index_size_bytes.ok_or(CliError {
                    msg: "Global secondary index size not found".to_string(),
                })?;

                let gsi_projection_type = gsi
                    .projection
                    .as_ref()
                    .and_then(|p| p.projection_type.as_ref())
                    .map(|p| p.as_str().to_string());

                let (
                    gsi_p_throughput_decreases_day,
                    gsi_p_throughput_read_units,
                    gsi_p_throughput_write_units,
                ) = gsi
                    .provisioned_throughput
                    .as_ref()
                    .map(|p| {
                        (
                            p.number_of_decreases_today,
                            p.read_capacity_units,
                            p.write_capacity_units,
                        )
                    })
                    .unwrap_or_default();

                let gsi_metadata = GsiMetadata {
                    gsi_name,
                    item_count: gsi_item_count,
                    projection_type: gsi_projection_type,
                    size_bytes: gsi_size_bytes,
                    p_throughput_decreases_day: gsi_p_throughput_decreases_day,
                    p_throughput_read_units: gsi_p_throughput_read_units,
                    p_throughput_write_units: gsi_p_throughput_write_units,
                };
                instances.push(DynamoDbResource {
                    id: table_name.to_string(),
                    metrics: vec![],
                    resource_type: ResourceType::DynamoDbGsi,
                    metadata: metadata.clone_with_gsi(gsi_metadata),
                    region: region.to_string(),
                    metric_period_seconds: 0,
                });
            }
            Ok::<Vec<DynamoDbResource>, CliError>(instances)
        })
        .unwrap_or_else(|| Ok(Vec::with_capacity(1)))?;

    resources.push(DynamoDbResource {
        id: table_name.to_string(),
        metrics: vec![],
        resource_type: ResourceType::DynamoDbTable,
        metadata,
        region: region.to_string(),
        metric_period_seconds: 0,
    });

    for mut resource in resources {
        resource
            .append_metrics(metrics_client, Arc::clone(&metrics_limiter))
            .await?;
        let ttl_enabled =
            is_ddb_ttl_enabled(ddb_client, &resource, Arc::clone(&describe_ttl_limiter)).await?;
        resource.metadata.ttl_enabled = ttl_enabled;
        sender
            .send(Resource::DynamoDb(resource))
            .await
            .map_err(|err| CliError {
                msg: format!("Failed to stream dynamodb resource to file: {}", err),
            })?;
    }

    Ok(())
}
