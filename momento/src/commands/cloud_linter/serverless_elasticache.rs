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
    let bar =
        ProgressBar::new_spinner().with_message("Describing Serverless ElastiCache resources");
    bar.enable_steady_tick(Duration::from_millis(100));
    bar.set_style(
        ProgressStyle::with_template("{spinner:.green} {pos:>7} {msg}")
            .expect("template should be valid")
            // For more spinners check out the cli-spinners project:
            // https://github.com/sindresorhus/cli-spinners/blob/master/spinners.json
            .tick_strings(&[
                "▹▹▹▹▹",
                "▸▹▹▹▹",
                "▹▸▹▹▹",
                "▹▹▸▹▹",
                "▹▹▹▸▹",
                "▹▹▹▹▸",
                "▪▪▪▪▪",
            ]),
    );
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
                        let futures = FuturesUnordered::new();
                        for cluster in clusters {
                            let metrics_client_clone = metrics_client.clone();
                            let region_clone = region.to_string().clone();
                            let sender_clone = sender.clone();
                            let metrics_limiter_clone = Arc::clone(&metrics_limiter);
                            let bar_clone = bar.clone();
                            let spawn = tokio::spawn(async move {
                                write_resource(
                                    cluster,
                                    metrics_client_clone,
                                    region_clone.as_str(),
                                    sender_clone,
                                    metrics_limiter_clone,
                                    bar_clone,
                                )
                                .await
                            });
                            futures.push(spawn);
                        }
                        let all_results = futures::future::join_all(futures).await;
                        for result in all_results {
                            match result {
                                // bubble up any cli errors that we came across
                                Ok(res) => res?,
                                Err(_) => {
                                    println!("failed to process serverless elasticache resources");
                                    return Err(CliError {
                                    msg: "failed to wait for all serverless elasticache resources to collect data".to_string(),
                                });
                                }
                            }
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
    bar.finish();

    Ok(())
}

async fn write_resource(
    cache: ServerlessCache,
    metrics_client: aws_sdk_cloudwatch::Client,
    region: &str,
    sender: Sender<Resource>,
    metrics_limiter: Arc<DefaultDirectRateLimiter>,
    bar: ProgressBar,
) -> Result<(), CliError> {
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

    let mut serverless_ec_resource = ServerlessElastiCacheResource {
        resource_type: ResourceType::ServerlessElastiCache,
        region: region.to_string(),
        id: cache_name,
        metrics: vec![],
        metric_period_seconds: 0,
        metadata,
    };
    serverless_ec_resource
        .append_metrics(&metrics_client, Arc::clone(&metrics_limiter))
        .await?;

    let resource = Resource::ServerlessElastiCache(serverless_ec_resource);
    sender.send(resource).await.map_err(|err| CliError {
        msg: format!("Failed to send serverless elasticache resource: {}", err),
    })?;
    bar.inc(1);

    Ok(())
}
