use std::collections::HashMap;
use std::sync::Arc;

use aws_sdk_cloudwatch::primitives::DateTime;
use aws_sdk_cloudwatch::types::Metric as CloudwatchMetric;
use aws_sdk_cloudwatch::types::{Dimension, MetricDataQuery, MetricStat};
use aws_sdk_cloudwatch::Client;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use governor::DefaultDirectRateLimiter;
use phf::Map;
use serde::{Deserialize, Serialize};

use crate::commands::cloud_linter::utils::rate_limit;
use crate::error::CliError;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub(crate) struct Metric {
    pub name: String,
    pub values: Vec<f64>,
}

pub(crate) struct MetricTarget {
    pub(crate) namespace: String,
    // a metric target should have either an expression or dimensions but not both
    pub(crate) expression: String,
    pub(crate) dimensions: HashMap<String, String>,
    pub(crate) targets: Map<&'static str, &'static [&'static str]>,
}

pub(crate) trait ResourceWithMetrics {
    fn create_metric_targets(&self) -> Result<Vec<MetricTarget>, CliError>;

    fn set_metrics(&mut self, metrics: Vec<Metric>);

    fn set_metric_period_seconds(&mut self, period: i32);
}

pub(crate) trait AppendMetrics {
    async fn append_metrics(
        &mut self,
        config: &Client,
        limiter: Arc<DefaultDirectRateLimiter>,
        start_millis: i64,
        end_millis: i64,
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
        start_millis: i64,
        end_millis: i64,
    ) -> Result<(), CliError> {
        let metric_targets = self.create_metric_targets()?;
        let mut metrics: Vec<Vec<Metric>> = Vec::new();
        let mut futures = FuturesUnordered::new();

        for target in metric_targets {
            let client = metrics_client.clone();
            let moved_limiter = Arc::clone(&limiter);
            let spawn = tokio::spawn(async move {
                query_metrics_for_target(&client, moved_limiter, start_millis, end_millis, target)
                    .await
            });
            futures.push(spawn);
        }
        while let Some(finished_future) = futures.next().await {
            match finished_future {
                Err(_e) => {
                    return Err(CliError {
                        msg: "failed to retrieve metrics from cloudwatch".to_string(),
                    })
                }
                Ok(result) => {
                    let resource_metrics = result?;
                    metrics.push(resource_metrics);
                }
            }
        }

        self.set_metrics(metrics.into_iter().flatten().collect());
        self.set_metric_period_seconds(60 * 60 * 24);

        Ok(())
    }
}

async fn query_metrics_for_target(
    client: &Client,
    limiter: Arc<DefaultDirectRateLimiter>,
    start_millis: i64,
    end_millis: i64,
    metric_target: MetricTarget,
) -> Result<Vec<Metric>, CliError> {
    let mut metric_results: Vec<Metric> = Vec::new();
    let dimensions: Vec<Dimension> = metric_target
        .dimensions
        .into_iter()
        .map(|(name, value)| Dimension::builder().name(name).value(value).build())
        .collect();
    let mut metric_data_query: MetricDataQuery;
    for (stat_type, metrics) in metric_target.targets.entries() {
        let mut metric_data_queries: Vec<MetricDataQuery> = Vec::with_capacity(metrics.len());
        for metric in *metrics {
            if metric_target.expression.is_empty() {
                metric_data_query = MetricDataQuery::builder()
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
            } else {
                let search_expression = format!(
                    "SEARCH(\' {} \', \'{}\')",
                    metric_target.expression, stat_type
                );
                metric_data_query = MetricDataQuery::builder()
                    .expression(search_expression)
                    .period(60 * 60 * 24)
                    .return_data(true)
                    .id(format!(
                        "{}_{}",
                        metric.to_lowercase(),
                        stat_type.to_lowercase()
                    ))
                    .build();
            }
            metric_data_queries.push(metric_data_query);
        }

        let mut metric_stream = client
            .get_metric_data()
            .start_time(DateTime::from_millis(start_millis))
            .end_time(DateTime::from_millis(end_millis))
            .set_metric_data_queries(Some(metric_data_queries))
            .into_paginator()
            .send();

        while let Some(result) = rate_limit(Arc::clone(&limiter), || metric_stream.next()).await {
            let result = match result {
                Ok(res) => res,
                Err(e) => {
                    println!("get_metric_data_error: {:?}", e);
                    return Err(CliError {
                        msg: "error from aws api while querying metrics".to_string(),
                    });
                }
            };
            // let result = result?;
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
