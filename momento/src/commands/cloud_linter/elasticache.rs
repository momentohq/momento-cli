use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use aws_config::SdkConfig;
use aws_sdk_elasticache::types::CacheCluster;
use futures::stream::FuturesUnordered;
use governor::DefaultDirectRateLimiter;
use indicatif::{ProgressBar, ProgressStyle};
use log::debug;
use phf::{phf_map, Map};
use serde::Serialize;
use tokio::sync::mpsc::Sender;

use crate::commands::cloud_linter::metrics::{Metric, MetricTarget, ResourceWithMetrics};
use crate::commands::cloud_linter::resource::{ElastiCacheResource, Resource, ResourceType};
use crate::commands::cloud_linter::utils::rate_limit;
use crate::error::CliError;

use super::metrics::AppendMetrics;

pub(crate) const CACHE_METRICS: Map<&'static str, &'static [&'static str]> = phf_map! {
        "Sum" => &[
            "NetworkBytesIn",
            "NetworkBytesOut",
            "GeoSpatialBasedCmds",
            "EvalBasedCmds",
            "GetTypeCmds",
            "HashBasedCmds",
            "JsonBasedCmds",
            "KeyBasedCmds",
            "ListBasedCmds",
            "SetBasedCmds",
            "SetTypeCmds",
            "StringBasedCmds",
            "PubSubBasedCmds",
            "SortedSetBasedCmds",
            "StreamBasedCmds",
            "ErrorCount",
        ],
        "Average" => &[
            "DB0AverageTTL",
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
pub(crate) struct ElastiCacheMetadata {
    #[serde(rename = "clusterId")]
    cluster_id: String,
    engine: String,
    #[serde(rename = "cacheNodeType")]
    cache_node_type: String,
    #[serde(rename = "preferredAz")]
    preferred_az: String,
    #[serde(rename = "clusterModeEnabled")]
    cluster_mode_enabled: bool,
}

impl ResourceWithMetrics for ElastiCacheResource {
    fn create_metric_targets(&self) -> Result<Vec<MetricTarget>, CliError> {
        match self.resource_type {
            ResourceType::ElastiCacheRedisNode => Ok(vec![MetricTarget {
                namespace: "AWS/ElastiCache".to_string(),
                expression: "".to_string(),
                dimensions: HashMap::from([
                    ("CacheClusterId".to_string(), self.id.clone()),
                    ("CacheNodeId".to_string(), "0001".to_string()),
                ]),
                targets: CACHE_METRICS,
            }]),
            ResourceType::ElastiCacheMemcachedNode => Ok(vec![MetricTarget {
                namespace: "AWS/ElastiCache".to_string(),
                expression: "".to_string(),
                dimensions: HashMap::from([
                    (
                        "CacheClusterId".to_string(),
                        self.metadata.cluster_id.clone(),
                    ),
                    ("CacheNodeId".to_string(), self.id.clone()),
                ]),
                targets: CACHE_METRICS,
            }]),
            ResourceType::ElastiCacheValkeyNode => Ok(vec![MetricTarget {
                namespace: "AWS/ElastiCache".to_string(),
                expression: "".to_string(),
                dimensions: HashMap::from([
                    ("CacheClusterId".to_string(), self.id.clone()),
                    ("CacheNodeId".to_string(), "0001".to_string()),
                ]),
                targets: CACHE_METRICS,
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

pub(crate) async fn process_elasticache_resources(
    config: &SdkConfig,
    control_plane_limiter: Arc<DefaultDirectRateLimiter>,
    metrics_limiter: Arc<DefaultDirectRateLimiter>,
    sender: Sender<Resource>,
    metrics_start_millis: i64,
    metrics_end_millis: i64,
    resource_filter: Option<ResourceType>,
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
        sender,
        region,
        metrics_start_millis,
        metrics_end_millis,
        resource_filter,
    )
    .await?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn process_resources(
    elasticache_client: &aws_sdk_elasticache::Client,
    metrics_client: &aws_sdk_cloudwatch::Client,
    control_plane_limiter: Arc<DefaultDirectRateLimiter>,
    metrics_limiter: Arc<DefaultDirectRateLimiter>,
    sender: Sender<Resource>,
    region: &str,
    metrics_start_millis: i64,
    metrics_end_millis: i64,
    resource_filter: Option<ResourceType>,
) -> Result<(), CliError> {
    let describe_bar = ProgressBar::new_spinner().with_message("Listing ElastiCache resources");
    describe_bar.enable_steady_tick(Duration::from_millis(100));
    let mut resources = describe_clusters(
        elasticache_client,
        control_plane_limiter,
        region,
        resource_filter,
    )
    .await?;
    describe_bar.finish();

    let process_bar =
        ProgressBar::new(resources.len() as u64).with_message("Processing ElastiCache resources");
    process_bar.set_style(
        ProgressStyle::with_template(" {pos:>7}/{len:7} {msg}").expect("invalid template"),
    );

    while !resources.is_empty() {
        let chunk: Vec<ElastiCacheResource> = resources
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
                    .append_metrics(
                        &metrics_client_clone,
                        metrics_limiter_clone,
                        metrics_start_millis,
                        metrics_end_millis,
                    )
                    .await?;

                let wrapped_resource = Resource::ElastiCache(resource);
                sender_clone
                    .send(wrapped_resource)
                    .await
                    .map_err(|err| CliError {
                        msg: format!("Failed to send elasticache resource: {}", err),
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
                    println!("failed to process elasticache resources");
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

async fn describe_clusters(
    elasticache_client: &aws_sdk_elasticache::Client,
    control_plane_limiter: Arc<DefaultDirectRateLimiter>,
    region: &str,
    resource_filter: Option<ResourceType>,
) -> Result<Vec<ElastiCacheResource>, CliError> {
    let mut clusters = Vec::new();
    let mut next_marker: Option<String> = None;
    loop {
        let response = rate_limit(Arc::clone(&control_plane_limiter), || {
            let mut req = elasticache_client
                .describe_cache_clusters()
                .show_cache_node_info(true);
            if let Some(marker) = &next_marker {
                req = req.marker(marker);
            }
            req.send()
        })
        .await?;

        if let Some(aws_clusters) = response.cache_clusters.as_ref() {
            clusters.extend_from_slice(aws_clusters);
        }

        next_marker = response.marker().map(String::from);
        if next_marker.is_none() {
            break;
        }
    }

    clusters
        .into_iter()
        .map(|cluster| convert_to_resources(cluster, region, resource_filter))
        .collect::<Result<Vec<_>, _>>()
        .map(|vec| vec.into_iter().flatten().collect())
}

fn convert_to_resources(
    cluster: CacheCluster,
    region: &str,
    resource_filter: Option<ResourceType>,
) -> Result<Vec<ElastiCacheResource>, CliError> {
    let mut resources = Vec::new();

    let cache_cluster_id = cluster.cache_cluster_id.ok_or(CliError {
        msg: "ElastiCache cluster has no ID".to_string(),
    })?;
    let cache_node_type = cluster.cache_node_type.ok_or(CliError {
        msg: "ElastiCache cluster has no node type".to_string(),
    })?;
    let preferred_az = cluster.preferred_availability_zone.ok_or(CliError {
        msg: "ElastiCache cluster has no preferred availability zone".to_string(),
    })?;

    let engine = cluster.engine.ok_or(CliError {
        msg: "ElastiCache cluster has no engine type".to_string(),
    })?;
    match engine.as_str() {
        "redis" => {
            if resource_filter.is_some()
                && resource_filter != Some(ResourceType::ElastiCacheRedisNode)
            {
                return Ok(vec![]);
            }

            let (cluster_id, cluster_mode_enabled) = cluster
                .replication_group_id
                .map(|replication_group_id| {
                    let trimmed_cluster_id = cache_cluster_id.clone();
                    let trimmed_cluster_id = trimmed_cluster_id
                        .trim_start_matches(&format!("{}-", replication_group_id));
                    let parts_len = trimmed_cluster_id.split('-').count();
                    (replication_group_id, parts_len == 2)
                })
                .unwrap_or_else(|| (cache_cluster_id.clone(), false));

            let metadata = ElastiCacheMetadata {
                cluster_id,
                engine,
                cache_node_type,
                preferred_az,
                cluster_mode_enabled,
            };

            let resource = ElastiCacheResource {
                resource_type: ResourceType::ElastiCacheRedisNode,
                region: region.to_string(),
                id: cache_cluster_id.clone(),
                metrics: vec![],
                metric_period_seconds: 0,
                metadata,
            };

            resources.push(resource);
        }
        "memcached" => {
            if resource_filter.is_some()
                && resource_filter != Some(ResourceType::ElastiCacheMemcachedNode)
            {
                return Ok(vec![]);
            }

            let metadata = ElastiCacheMetadata {
                cluster_id: cache_cluster_id,
                engine,
                cache_node_type,
                preferred_az,
                cluster_mode_enabled: false,
            };

            if let Some(cache_nodes) = cluster.cache_nodes {
                for node in cache_nodes {
                    let cache_node_id = node.cache_node_id.ok_or(CliError {
                        msg: "Cache node has no ID".to_string(),
                    })?;
                    let resource = ElastiCacheResource {
                        resource_type: ResourceType::ElastiCacheMemcachedNode,
                        region: region.to_string(),
                        id: cache_node_id,
                        metrics: vec![],
                        metric_period_seconds: 0,
                        metadata: metadata.clone(),
                    };
                    resources.push(resource)
                }
            }
        }
        "valkey" => {
            if resource_filter.is_some()
                && resource_filter != Some(ResourceType::ElastiCacheValkeyNode)
            {
                return Ok(vec![]);
            }

            let metadata = ElastiCacheMetadata {
                cluster_id: cache_cluster_id,
                engine,
                cache_node_type,
                preferred_az,
                cluster_mode_enabled: false,
            };

            if let Some(cache_nodes) = cluster.cache_nodes {
                for node in cache_nodes {
                    let cache_node_id = node.cache_node_id.ok_or(CliError {
                        msg: "Cache node has no ID".to_string(),
                    })?;
                    let resource = ElastiCacheResource {
                        resource_type: ResourceType::ElastiCacheValkeyNode,
                        region: region.to_string(),
                        id: cache_node_id,
                        metrics: vec![],
                        metric_period_seconds: 0,
                        metadata: metadata.clone(),
                    };
                    resources.push(resource)
                }
            }
        }
        _ => {
            debug!("Unknown engine: {}", engine.as_str());
            return Ok(vec![]);
        }
    };

    Ok(resources)
}
