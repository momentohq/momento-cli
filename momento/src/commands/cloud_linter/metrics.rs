use std::collections::HashMap;
use std::sync::Arc;

use aws_sdk_cloudwatch::primitives::DateTime;
use aws_sdk_cloudwatch::types::Metric as CloudwatchMetric;
use aws_sdk_cloudwatch::types::{Dimension, MetricDataQuery, MetricStat};
use aws_sdk_cloudwatch::Client;
use chrono::{Duration, Utc};
use governor::DefaultDirectRateLimiter;
use phf::Map;
use serde::{Deserialize, Serialize};

use crate::commands::cloud_linter::utils::rate_limit;
use crate::error::CliError;

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct Metric {
    pub name: String,
    pub values: Vec<f64>,
}

pub(crate) struct MetricTarget {
    pub(crate) prefix: String,
    pub(crate) namespace: String,
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
        let metric_targets = self.create_metric_targets()?;
        let mut metrics: Vec<Vec<Metric>> = Vec::new();
        for target in metric_targets {
            metrics.push(
                query_metrics_for_target(metrics_client, Arc::clone(&limiter), target).await?
            );
        }
        self.set_metrics(metrics.into_iter().flatten().collect());
        self.set_metric_period_seconds(60 * 60 * 24);

        Ok(())
    }
}

async fn query_metrics_for_target(
    client: &Client,
    limiter: Arc<DefaultDirectRateLimiter>,
    metric_target: MetricTarget,
) -> Result<Vec<Metric>, CliError> {
    let mut metric_results: Vec<Metric> = Vec::new();
    if metric_target.expression.is_empty() {
        println!("No expression for metric target");
        return Ok(metric_results);
    } else {
        println!("Will query metric expression: {}", metric_target.expression);
    }
    // let dimensions: Vec<Dimension> = metric_target
    //     .dimensions
    //     .into_iter()
    //     .map(|(name, value)| Dimension::builder().name(name).value(value).build())
    //     .collect();
    for (stat_type, metrics) in metric_target.targets.entries() {
        let mut metric_data_queries: Vec<MetricDataQuery> = Vec::with_capacity(metrics.len());
        for metric in *metrics {
            println!("Querying metric: {}", metric);
            println!("expression is {}", metric_target.expression);
            let search_expression = format!("SEARCH(\' {} \', \'{}\')", metric_target.expression, "Sum");
            println!("search expression is {}", search_expression);
            println!("stat type is {}", stat_type);
            let metric_data_query = MetricDataQuery::builder()
                // .metric_stat(
                //     MetricStat::builder()
                //         .metric(
                //             CloudwatchMetric::builder()
                //                 .metric_name(metric.to_string())
                //                 .namespace(metric_target.namespace.clone())
                //                 .set_dimensions(Some(dimensions.clone()))
                //                 .build(),
                //         )
                //         .period(60 * 60 * 24)
                //         .stat(stat_type.to_string())
                //         .build(),
                // )
                .expression(search_expression)
                .period(60 * 60 * 24)
                .return_data(true)
                .id(format!(
                    "{}{}_{}",
                    metric_target.prefix,
                    metric.to_lowercase(),
                    stat_type.to_lowercase()
                ))
                .build();
            metric_data_queries.push(metric_data_query);
        }

        println!("Querying metrics");
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
            println!("Got page of metrics {:?}", result);
            let result = result?;
            println!("assigned result");
            println!("Got {} metric data results", result.clone().metric_data_results.expect("hi!").len());
            if let Some(mdr_vec) = result.metric_data_results {
                println!("Got {} metric data results", mdr_vec.len());
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
