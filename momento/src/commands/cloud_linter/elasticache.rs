use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use aws_config::SdkConfig;
use aws_sdk_elasticache::types::CacheCluster;
use governor::DefaultDirectRateLimiter;
use indicatif::{ProgressBar, ProgressStyle};
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
    let describe_bar = ProgressBar::new_spinner().with_message("Listing ElastiCache resources");
    describe_bar.enable_steady_tick(Duration::from_millis(100));
    let resources = describe_clusters(elasticache_client, control_plane_limiter, region).await?;
    describe_bar.finish();

    let process_bar =
        ProgressBar::new(resources.len() as u64).with_message("Processing ElastiCache resources");
    process_bar.set_style(
        ProgressStyle::with_template(" {pos:>7}/{len:7} {msg}").expect("invalid template"),
    );

    for resource in resources {
        match resource {
            Resource::ElastiCache(mut er) => {
                er.append_metrics(metrics_client, Arc::clone(&metrics_limiter))
                    .await?;
                sender
                    .send(Resource::ElastiCache(er))
                    .await
                    .map_err(|err| CliError {
                        msg: format!("Failed to send elasticache resource: {}", err),
                    })?;
                process_bar.inc(1);
            }
            _ => {
                return Err(CliError {
                    msg: "Invalid resource type".to_string(),
                });
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
) -> Result<Vec<Resource>, CliError> {
    let mut resources = Vec::new();
    let mut elasticache_stream = elasticache_client
        .describe_cache_clusters()
        .show_cache_node_info(true)
        .into_paginator()
        .send();

    while let Some(result) = rate_limit(Arc::clone(&control_plane_limiter), || {
        elasticache_stream.next()
    })
    .await
    {
        match result {
            Ok(result) => {
                if let Some(aws_clusters) = result.cache_clusters {
                    let mut chunks = Vec::new();
                    for chunk in aws_clusters.chunks(10) {
                        chunks.push(chunk.to_owned());
                    }
                    for clusters in chunks {
                        for cluster in clusters {
                            let cluster_resources = convert_to_resources(cluster, region).await?;
                            resources.extend(cluster_resources);
                        }
                    }
                }
            }
            Err(err) => {
                return Err(CliError {
                    msg: format!("Failed to describe cache clusters: {}", err),
                });
            }
        }
    }

    Ok(resources)
}

async fn convert_to_resources(
    cluster: CacheCluster,
    region: &str,
) -> Result<Vec<Resource>, CliError> {
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

            let resource = Resource::ElastiCache(ElastiCacheResource {
                resource_type: ResourceType::ElastiCacheRedisNode,
                region: region.to_string(),
                id: cache_cluster_id.clone(),
                metrics: vec![],
                metric_period_seconds: 0,
                metadata,
            });

            resources.push(resource);
        }
        "memcached" => {
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
                    let resource = Resource::ElastiCache(ElastiCacheResource {
                        resource_type: ResourceType::ElastiCacheMemcachedNode,
                        region: region.to_string(),
                        id: cache_node_id,
                        metrics: vec![],
                        metric_period_seconds: 0,
                        metadata: metadata.clone(),
                    });
                    resources.push(resource)
                }
            }
        }
        _ => {
            return Err(CliError {
                msg: format!("Unsupported engine: {}", engine),
            });
        }
    };

    Ok(resources)
}
