use std::sync::Arc;

use aws_config::SdkConfig;
use aws_sdk_elasticache::types::CacheCluster;
use governor::DefaultDirectRateLimiter;
use serde::{Deserialize, Serialize};

use crate::commands::cloud_linter::utils::rate_limit;
use crate::error::CliError;
use crate::utils::console::console_info;

#[derive(Serialize, Deserialize)]
pub(crate) struct ElastiCacheMetadata {
    cluster_id: String,
    engine: String,
    cache_node_type: String,
    preferred_az: String,
    cluster_mode_enabled: bool,
}

pub(crate) async fn get_elasticache_metadata(
    config: &SdkConfig,
    limiter: Arc<DefaultDirectRateLimiter>,
) -> Result<Vec<ElastiCacheMetadata>, CliError> {
    console_info!("Describing ElastiCache clusters");
    let elasticache_client = aws_sdk_elasticache::Client::new(config);
    list_table_names(&elasticache_client, limiter)
        .await?
        .into_iter()
        .map(ElastiCacheMetadata::try_from)
        .collect()
}

async fn list_table_names(
    elasticache_client: &aws_sdk_elasticache::Client,
    limiter: Arc<DefaultDirectRateLimiter>,
) -> Result<Vec<CacheCluster>, CliError> {
    let mut elasticache_clusters = Vec::new();
    let mut elasticache_stream = elasticache_client
        .describe_cache_clusters()
        .show_cache_node_info(true)
        .into_paginator()
        .send();

    while let Some(result) = rate_limit(Arc::clone(&limiter), || elasticache_stream.next()).await {
        match result {
            Ok(result) => {
                if let Some(clusters) = result.cache_clusters {
                    elasticache_clusters.extend(clusters);
                }
            }
            Err(err) => {
                return Err(CliError {
                    msg: format!("Failed to describe cache clusters: {}", err),
                })
            }
        }
    }

    Ok(elasticache_clusters)
}

impl TryFrom<CacheCluster> for ElastiCacheMetadata {
    type Error = CliError;

    fn try_from(value: CacheCluster) -> Result<Self, Self::Error> {
        let cache_cluster_id = value.cache_cluster_id.ok_or(CliError {
            msg: "ElastiCache cluster has no ID".to_string(),
        })?;
        let cache_node_type = value.cache_node_type.ok_or(CliError {
            msg: "ElastiCache cluster has no node type".to_string(),
        })?;
        let preferred_az = value.preferred_availability_zone.ok_or(CliError {
            msg: "ElastiCache cluster has no preferred availability zone".to_string(),
        })?;

        let engine = value.engine.ok_or(CliError {
            msg: "ElastiCache cluster has no node type".to_string(),
        })?;
        match engine.as_str() {
            "redis" => {
                let (cluster_id, cluster_mode_enabled) = value
                    .replication_group_id
                    .map(|replication_group_id| {
                        let trimmed_cluster_id = cache_cluster_id
                            .trim_start_matches(&format!("{}-", replication_group_id));
                        let parts_len = trimmed_cluster_id.split('-').count();
                        (replication_group_id, parts_len == 2)
                    })
                    .unwrap_or_else(|| (cache_cluster_id, false));

                Ok(ElastiCacheMetadata {
                    cluster_id,
                    engine,
                    cache_node_type,
                    preferred_az,
                    cluster_mode_enabled,
                })
            }
            "memcached" => Ok(ElastiCacheMetadata {
                cluster_id: cache_cluster_id,
                engine,
                cache_node_type,
                preferred_az,
                cluster_mode_enabled: false,
            }),
            _ => Err(CliError {
                msg: format!("Unsupported engine: {}", engine),
            }),
        }
    }
}
