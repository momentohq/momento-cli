use std::sync::Arc;

use aws_config::SdkConfig;
use aws_sdk_dynamodb::types::{TimeToLiveDescription, TimeToLiveStatus};
use governor::DefaultDirectRateLimiter;
use serde::{Deserialize, Serialize};

use crate::commands::cloud_linter::utils::rate_limit;
use crate::error::CliError;
use crate::utils::console::console_info;

#[derive(Serialize, Deserialize)]
pub(crate) struct DynamoDbMetadata {
    avg_item_size_bytes: i64,
    billing_mode: Option<String>,
    gsi_count: i64,
    item_count: i64,
    ttl_enabled: bool,
    is_global_table: bool,
    lsi_count: i64,
    table_class: Option<String>,
    table_size_bytes: i64,
    p_throughput_decreases_day: Option<i64>,
    p_throughput_read_units: Option<i64>,
    p_throughput_write_units: Option<i64>,
}

pub(crate) async fn get_ddb_metadata(
    config: &SdkConfig,
    limiter: Arc<DefaultDirectRateLimiter>,
) -> Result<Vec<DynamoDbMetadata>, CliError> {
    let ddb_client = aws_sdk_dynamodb::Client::new(&config);

    console_info!("Listing Dynamo DB tables");
    let table_names = list_table_names(&ddb_client, Arc::clone(&limiter)).await?;

    console_info!("Describing tables");
    let mut table_info = Vec::with_capacity(table_names.len());
    for table_name in table_names {
        let metadata = describe_table(&ddb_client, &table_name, Arc::clone(&limiter)).await?;
        table_info.push(metadata);
    }

    Ok(table_info)
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
                })
            }
        }
    }

    Ok(table_names)
}

async fn describe_table(
    ddb_client: &aws_sdk_dynamodb::Client,
    table_name: &str,
    limiter: Arc<DefaultDirectRateLimiter>,
) -> Result<DynamoDbMetadata, CliError> {
    let ttl = rate_limit(Arc::clone(&limiter), || async {
        ddb_client
            .describe_time_to_live()
            .table_name(table_name)
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

    let description = rate_limit(Arc::clone(&limiter), || async {
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
        .map(|gsi| gsi.len() as i64)
        .unwrap_or_default();

    let lsi_count = table
        .local_secondary_indexes
        .map(|lsi| lsi.len() as i64)
        .unwrap_or_default();

    let is_global_table = table.global_table_version.is_some();

    let (p_throughput_decreases_day, p_throughput_read_units, p_throughput_write_units) = table
        .provisioned_throughput
        .map(|p| {
            (
                p.number_of_decreases_today,
                p.read_capacity_units,
                p.write_capacity_units,
            )
        })
        .unwrap_or_default();

    Ok(DynamoDbMetadata {
        avg_item_size_bytes,
        billing_mode,
        gsi_count,
        item_count,
        ttl_enabled,
        is_global_table,
        lsi_count,
        table_class,
        table_size_bytes,
        p_throughput_decreases_day,
        p_throughput_read_units,
        p_throughput_write_units,
    })
}
