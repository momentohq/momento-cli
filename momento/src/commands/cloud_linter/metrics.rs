use std::collections::HashMap;
use std::sync::Arc;

use crate::commands::cloud_linter::resource::Resource;
use crate::commands::cloud_linter::utils::rate_limit;
use aws_config::SdkConfig;
use aws_sdk_cloudwatch::primitives::DateTime;
use aws_sdk_cloudwatch::types::Metric as CloudwatchMetric;
use aws_sdk_cloudwatch::types::{Dimension, MetricDataQuery, MetricStat};
use aws_sdk_cloudwatch::Client;
use chrono::{Duration, Utc};
use governor::DefaultDirectRateLimiter;
use phf::Map;
use serde::{Deserialize, Serialize};

use crate::error::CliError;
use crate::utils::console::console_info;

#[derive(Serialize, Deserialize)]
pub(crate) struct Metric {
    name: String,
    values: Vec<f64>,
}

pub(crate) struct MetricTarget {
    pub(crate) namespace: String,
    pub(crate) dimensions: HashMap<String, String>,
    pub(crate) targets: Map<&'static str, &'static [&'static str]>,
}

pub(crate) trait ResourceWithMetrics {
    fn create_metric_target(&self) -> Result<MetricTarget, CliError>;

    fn set_metrics(&mut self, metrics: Vec<Metric>);

    fn set_metric_period_seconds(&mut self, period: i32);
}

pub(crate) trait AppendMetrics {
    async fn append_metrics(
        &mut self,
        config: &Client,
        limiter: Arc<DefaultDirectRateLimiter>,
    ) -> Result<(), CliError>;
}

impl<T> AppendMetrics for T
where
    T: ResourceWithMetrics,
{
    async fn append_metrics(
        &mut self,
        metrics_client: &Client,
        limiter: Arc<DefaultDirectRateLimiter>,
    ) -> Result<(), CliError> {
        let metric_target = self.create_metric_target()?;
        let metrics =
            query_metrics_for_target(&metrics_client, Arc::clone(&limiter), metric_target).await?;
        self.set_metrics(metrics);
        self.set_metric_period_seconds(60 * 60 * 24);

        Ok(())
    }
}

pub(crate) async fn append_metrics_to_resources(
    config: &SdkConfig,
    limiter: Arc<DefaultDirectRateLimiter>,
    mut resources: Vec<Resource>,
) -> Result<Vec<Resource>, CliError> {
    console_info!("Getting metrics...");
    let metrics_client = Client::new(config);

    for resource in &mut resources {
        match resource {
            Resource::DynamoDb(dynamodb_resource) => {
                dynamodb_resource
                    .append_metrics(&metrics_client, Arc::clone(&limiter))
                    .await?;
            }
            Resource::ElastiCache(elasticache_resource) => {
                elasticache_resource
                    .append_metrics(&metrics_client, Arc::clone(&limiter))
                    .await?;
            }
        }
    }

    Ok(resources)
}

async fn query_metrics_for_target(
    client: &Client,
    limiter: Arc<DefaultDirectRateLimiter>,
    metric_target: MetricTarget,
) -> Result<Vec<Metric>, CliError> {
    let mut metric_results: Vec<Metric> = Vec::new();
    let dimensions: Vec<Dimension> = metric_target
        .dimensions
        .into_iter()
        .map(|(name, value)| Dimension::builder().name(name).value(value).build())
        .collect();
    for (stat_type, metrics) in metric_target.targets.entries() {
        let mut metric_data_queries: Vec<MetricDataQuery> = Vec::with_capacity(metrics.len());
        for metric in *metrics {
            let metric_data_query = MetricDataQuery::builder()
                .metric_stat(
                    MetricStat::builder()
                        .metric(
                            CloudwatchMetric::builder()
                                .metric_name(metric.to_string())
                                .namespace(metric_target.namespace.clone())
                                .set_dimensions(Some(dimensions.clone()))
                                .build(),
                        )
                        .period(60 * 60 * 24)
                        .stat(stat_type.to_string())
                        .build(),
                )
                .id(format!(
                    "{}_{}",
                    metric.to_lowercase(),
                    stat_type.to_lowercase()
                ))
                .build();
            metric_data_queries.push(metric_data_query);
        }

        let mut metric_stream = client
            .get_metric_data()
            .start_time(DateTime::from_millis(
                (Utc::now() - Duration::days(30)).timestamp_millis(),
            ))
            .end_time(DateTime::from_millis(Utc::now().timestamp_millis()))
            .set_metric_data_queries(Some(metric_data_queries))
            .into_paginator()
            .send();

        while let Some(result) = rate_limit(Arc::clone(&limiter), || metric_stream.next()).await {
            let result = result?;
            if let Some(mdr_vec) = result.metric_data_results {
                for mdr in mdr_vec {
                    let name = mdr.id.ok_or_else(|| CliError {
                        msg: "Metric has no id".to_string(),
                    })?;
                    let values = mdr.values.ok_or_else(|| CliError {
                        msg: "Metric has no values".to_string(),
                    })?;
                    metric_results.push(Metric { name, values });
                }
            }
        }
    }

    Ok(metric_results)
}
