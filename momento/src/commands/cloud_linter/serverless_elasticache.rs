use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use aws_config::SdkConfig;
use aws_sdk_elasticache::types::{
    CacheUsageLimits, DataStorage, DataStorageUnit, EcpuPerSecond, ServerlessCache,
};
use futures::stream::FuturesUnordered;
use governor::DefaultDirectRateLimiter;
use indicatif::{ProgressBar, ProgressStyle};
use phf::{phf_map, Map};
use serde::Serialize;
use tokio::sync::mpsc::Sender;

use crate::commands::cloud_linter::metrics::{Metric, MetricTarget, ResourceWithMetrics};
use crate::commands::cloud_linter::resource::{
    Resource, ResourceType, ServerlessElastiCacheResource,
};
use crate::commands::cloud_linter::utils::rate_limit;
use crate::error::CliError;

use super::metrics::AppendMetrics;

pub(crate) const SERVERLESS_CACHE_METRICS: Map<&'static str, &'static [&'static str]> = phf_map! {
        "Sum" => &[
            "NetworkBytesIn",
            "NetworkBytesOut",
            "GeoSpatialBasedCmds",
            "EvalBasedCmds",
            "EvalBasedCmdsECPUs",
            "GetTypeCmds",
            "GetTypeCmdsECPUs" ,
            "HashBasedCmds",
            "HashBasedCmdsECPUs",
            "JsonBasedCmds",
            "JsonBasedCmdsECPUs",
            "KeyBasedCmds",
            "KeyBasedCmdsECPUs",
            "ListBasedCmds",
            "ListBasedCmdsECPUs",
            "SetBasedCmds",
            "SetBasedCmdsECPUs",
            "SetTypeCmds",
            "SetTypeCmdsECPUs",
            "StringBasedCmds",
            "StringBasedCmdsECPUs",
            "PubSubBasedCmds",
            "PubSubBasedCmdsECPUs",
            "SortedSetBasedCmds",
            "SortedSetBasedCmdsECPUs",
            "StreamBasedCmds",
            "StreamBasedCmdsECPUs",
            "ElastiCacheProcessingUnits"
        ],
        "Average" => &[
            "DB0AverageTTL",
            "ElastiCacheProcessingUnits"
        ],
        "Maximum" => &[
            "CurrConnections",
            "NewConnections",
            "EngineCPUUtilization",
            "CPUUtilization",
            "FreeableMemory",
            "BytesUsedForCache",
            "DatabaseMemoryUsagePercentage",
            "CurrItems",
            "KeysTracked",
            "Evictions",
            "CacheHitRate",
        ],
};

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub(crate) struct ServerlessElastiCacheMetadata {
    name: String,
    engine: String,
    #[serde(rename = "maxDataStorageGB")]
    max_data_storage_gb: i32,
    #[serde(rename = "dataStorageUnit")]
    data_storage_unit: String,
    #[serde(rename = "maxEcpuPerSecond")]
    max_ecpu_per_second: i32,
    #[serde(rename = "snapshotRetentionLimit")]
    snapshot_retention_limit: i32,
    #[serde(rename = "dailySnapshotTime")]
    daily_snapshot_time: String,
    #[serde(rename = "userGroupId")]
    user_group_id: String,
    #[serde(rename = "engineVersion")]
    engine_version: String,
}

impl ResourceWithMetrics for ServerlessElastiCacheResource {
    fn create_metric_targets(&self) -> Result<Vec<MetricTarget>, CliError> {
        match self.resource_type {
            ResourceType::ServerlessElastiCache => Ok(vec![MetricTarget {
                namespace: "AWS/ElastiCache".to_string(),
                expression: "".to_string(),
                dimensions: HashMap::from([
                    // the cache id for a serverless elasticache cluster is just the cache name
                    ("CacheClusterId".to_string(), self.id.clone()),
                ]),
                targets: SERVERLESS_CACHE_METRICS,
            }]),
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

pub(crate) async fn process_serverless_elasticache_resources(
    config: &SdkConfig,
    control_plane_limiter: Arc<DefaultDirectRateLimiter>,
    metrics_limiter: Arc<DefaultDirectRateLimiter>,
    sender: Sender<Resource>,
) -> Result<(), CliError> {
    let region = config.region().map(|r| r.as_ref()).ok_or(CliError {
        msg: "No region configured for client".to_string(),
    })?;

    let elasticache_client = aws_sdk_elasticache::Client::new(config);
    let metrics_client = aws_sdk_cloudwatch::Client::new(config);
    process_resources(
        &elasticache_client,
        &metrics_client,
        control_plane_limiter,
        metrics_limiter,
        region,
        sender,
    )
    .await?;

    Ok(())
}

async fn process_resources(
    elasticache_client: &aws_sdk_elasticache::Client,
    metrics_client: &aws_sdk_cloudwatch::Client,
    control_plane_limiter: Arc<DefaultDirectRateLimiter>,
    metrics_limiter: Arc<DefaultDirectRateLimiter>,
    region: &str,
    sender: Sender<Resource>,
) -> Result<(), CliError> {
    let describe_bar =
        ProgressBar::new_spinner().with_message("Listing Serverless ElastiCache resources");
    describe_bar.enable_steady_tick(Duration::from_millis(100));
    let mut resources = describe_caches(elasticache_client, control_plane_limiter, region).await?;
    describe_bar.finish();

    let process_bar = ProgressBar::new(resources.len() as u64)
        .with_message("Processing Serverless ElastiCache resources");
    process_bar.set_style(
        ProgressStyle::with_template(" {pos:>7}/{len:7} {msg}").expect("invalid template"),
    );

    while !resources.is_empty() {
        let chunk: Vec<ServerlessElastiCacheResource> = resources
            .drain(..std::cmp::min(10, resources.len()))
            .collect();

        let futures = FuturesUnordered::new();
        for mut resource in chunk {
            let metrics_limiter_clone = Arc::clone(&metrics_limiter);
            let sender_clone = sender.clone();
            let process_bar_clone = process_bar.clone();
            let metrics_client_clone = metrics_client.clone();

            futures.push(tokio::spawn(async move {
                resource
                    .append_metrics(&metrics_client_clone, metrics_limiter_clone)
                    .await?;

                let wrapped_resource = Resource::ServerlessElastiCache(resource);
                sender_clone
                    .send(wrapped_resource)
                    .await
                    .map_err(|err| CliError {
                        msg: format!("Failed to send serverless elasticache resource: {}", err),
                    })?;
                process_bar_clone.inc(1);
                Ok::<(), CliError>(())
            }));
        }

        let all_results = futures::future::join_all(futures).await;
        for result in all_results {
            match result {
                // bubble up any cli errors that we came across
                Ok(res) => res?,
                Err(_) => {
                    println!("failed to process serverless elasticache resources");
                    return Err(CliError {
                        msg: "failed to wait for all elasticache resources to collect data"
                            .to_string(),
                    });
                }
            }
        }
    }

    process_bar.finish();
    Ok(())
}

async fn describe_caches(
    elasticache_client: &aws_sdk_elasticache::Client,
    control_plane_limiter: Arc<DefaultDirectRateLimiter>,
    region: &str,
) -> Result<Vec<ServerlessElastiCacheResource>, CliError> {
    let mut resources = Vec::new();
    let mut elasticache_stream = elasticache_client
        .describe_serverless_caches()
        .into_paginator()
        .send();

    while let Some(result) = rate_limit(Arc::clone(&control_plane_limiter), || {
        elasticache_stream.next()
    })
    .await
    {
        match result {
            Ok(result) => {
                if let Some(aws_caches) = result.serverless_caches {
                    let mut chunks = Vec::new();
                    for chunk in aws_caches.chunks(10) {
                        chunks.push(chunk.to_owned());
                    }
                    for clusters in chunks {
                        for cluster in clusters {
                            resources.push(convert_to_resource(cluster, region).await?);
                        }
                    }
                }
            }
            Err(err) => {
                return Err(CliError {
                    msg: format!("Failed to describe serverless caches: {}", err),
                });
            }
        }
    }
    Ok(resources)
}

async fn convert_to_resource(
    cache: ServerlessCache,
    region: &str,
) -> Result<ServerlessElastiCacheResource, CliError> {
    let cache_name = cache.serverless_cache_name.unwrap_or_default();
    let engine = cache.engine.unwrap_or_default();
    let user_group_id = cache.user_group_id.unwrap_or_default();
    let snapshot_retention_limit = cache.snapshot_retention_limit.unwrap_or(0);
    let daily_snapshot_time = cache.daily_snapshot_time.unwrap_or_default();

    let cache_usage_limits = cache
        .cache_usage_limits
        .unwrap_or(CacheUsageLimits::builder().build());
    // By default, every Serverless cache can scale to a maximum of 5 TBs of data storage and 15,000,000 ECPUs per second. To control costs, you can choose to set lower usage limits so that your cache will scale to a lower maximum.
    //
    // When a maximum Memory usage limit is set and your cache hits that limit, then ElastiCache Serverless will begin to evict data, to reject new writes with an Out of Memory error, or both.
    //
    // When a maximum ECPUs/second limit is set and your cache hits that limit, then ElastiCache Serverless will begin throttling or rejecting requests.
    let data_storage = cache_usage_limits.data_storage.unwrap_or(
        DataStorage::builder()
            .set_maximum(Some(5_000))
            .set_unit(Some(DataStorageUnit::Gb))
            .build(),
    );

    let ecpu = cache_usage_limits.ecpu_per_second.unwrap_or(
        EcpuPerSecond::builder()
            .set_maximum(Some(15_000_000))
            .build(),
    );
    let max_data_storage_gb = data_storage.maximum.unwrap_or(5_000);
    let data_storage_unit = data_storage.unit.unwrap_or(DataStorageUnit::Gb);

    let metadata = ServerlessElastiCacheMetadata {
        name: cache_name.clone(),
        engine,
        max_data_storage_gb,
        max_ecpu_per_second: ecpu.maximum.unwrap_or_default(),
        snapshot_retention_limit,
        daily_snapshot_time,
        user_group_id,
        data_storage_unit: data_storage_unit.to_string(),
        engine_version: cache.full_engine_version.unwrap_or_default(),
    };

    Ok(ServerlessElastiCacheResource {
        resource_type: ResourceType::ServerlessElastiCache,
        region: region.to_string(),
        id: cache_name,
        metrics: vec![],
        metric_period_seconds: 0,
        metadata,
    })
}
