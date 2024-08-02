use crate::commands::cloud_linter::metrics::{
    AppendMetrics, Metric, MetricTarget, ResourceWithMetrics,
};
use crate::commands::cloud_linter::resource::{ApiGatewayResource, Resource, ResourceType};
use crate::commands::cloud_linter::utils::rate_limit;
use crate::error::CliError;
use aws_config::SdkConfig;
use aws_sdk_apigateway::types::RestApi;
use governor::DefaultDirectRateLimiter;
use indicatif::{ProgressBar, ProgressStyle};
use phf::{phf_map, Map};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

const API_GATEWAY_METRICS: Map<&'static str, &'static [&'static str]> = phf_map! {
    "Sum" => &[
        "CacheHitCount",
        "CacheMissCount",
        "Count",
        "IntegrationLatency",
        "Latency",
    ],
    "Average" => &[
        "CacheHitCount",
        "CacheMissCount",
        "Count",
        "IntegrationLatency",
        "Latency",
    ],
    "Maximum" => &[
        "CacheHitCount",
        "CacheMissCount",
        "Count",
        "IntegrationLatency",
        "Latency",
    ],
};

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub(crate) struct ApiGatewayMetadata {
    #[serde(rename = "name")]
    name: String,
}

impl ResourceWithMetrics for ApiGatewayResource {
    fn create_metric_targets(&self) -> Result<Vec<MetricTarget>, CliError> {
        let targets = vec![MetricTarget {
            namespace: "AWS/ApiGateway".to_string(),
            expression: "".to_string(),
            dimensions: HashMap::from([("ApiName".to_string(), self.metadata.name.clone())]),
            targets: API_GATEWAY_METRICS,
        }];
        match self.resource_type {
            ResourceType::ApiGateway => Ok(targets),
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

#[allow(clippy::too_many_arguments)]
pub(crate) async fn process_api_gateway_resources(
    config: &SdkConfig,
    control_plane_limiter: Arc<DefaultDirectRateLimiter>,
    metrics_limiter: Arc<DefaultDirectRateLimiter>,
    sender: Sender<Resource>,
    metrics_start_millis: i64,
    metrics_end_millis: i64,
) -> Result<(), CliError> {
    let region = config.region().map(|r| r.as_ref()).ok_or(CliError {
        msg: "No region configured for client".to_string(),
    })?;
    let apig_client = aws_sdk_apigateway::Client::new(config);
    let metrics_client = aws_sdk_cloudwatch::Client::new(config);

    let list_apis_bar = ProgressBar::new_spinner().with_message("Listing API Gateway resources");
    list_apis_bar.enable_steady_tick(std::time::Duration::from_millis(100));

    let mut apis = Vec::new();
    let mut resp_stream = apig_client.get_rest_apis().into_paginator().send();
    while let Some(result) =
        rate_limit(Arc::clone(&control_plane_limiter), || resp_stream.next()).await
    {
        match result {
            Ok(result) => {
                apis.extend(result.items.unwrap_or_default());
            }
            Err(e) => {
                return Err(CliError {
                    msg: format!("Failed to list API Gateway resources: {}", e),
                });
            }
        }
    }
    list_apis_bar.finish();
    process_apis(
        apig_client.clone(),
        &metrics_client,
        &metrics_limiter,
        region,
        metrics_start_millis,
        metrics_end_millis,
        &apis,
        sender,
    )
    .await?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn process_apis(
    apig_client: aws_sdk_apigateway::Client,
    metrics_client: &aws_sdk_cloudwatch::Client,
    metrics_limiter: &Arc<DefaultDirectRateLimiter>,
    region: &str,
    metrics_start_millis: i64,
    metrics_end_millis: i64,
    apis: &[RestApi],
    sender: Sender<Resource>,
) -> Result<(), CliError> {
    let mut resources: Vec<Resource> = Vec::with_capacity(apis.len());
    let get_apis_bar =
        ProgressBar::new((apis.len() * 2) as u64).with_message("Processing API Gateway resources");
    get_apis_bar.set_style(
        ProgressStyle::with_template(" {pos:>7}/{len:7} {msg}").expect("invalid template"),
    );
    for api in apis {
        let the_api = apig_client
            .get_rest_api()
            .rest_api_id(api.id.clone().unwrap_or_default())
            .send()
            .await?;

        let metadata = ApiGatewayMetadata {
            name: the_api.name.clone().unwrap_or_default(),
        };

        let apig_resource = ApiGatewayResource {
            resource_type: ResourceType::ApiGateway,
            region: region.to_string(),
            id: the_api.id.clone().unwrap_or_default(),
            metrics: vec![],
            metric_period_seconds: 0,
            metadata,
        };
        resources.push(Resource::ApiGateway(apig_resource));
        get_apis_bar.inc(1);
    }

    for resource in resources {
        match resource {
            Resource::ApiGateway(mut apig_resource) => {
                apig_resource
                    .append_metrics(
                        metrics_client,
                        Arc::clone(metrics_limiter),
                        metrics_start_millis,
                        metrics_end_millis,
                    )
                    .await?;
                sender
                    .send(Resource::ApiGateway(apig_resource))
                    .await
                    .map_err(|_| CliError {
                        msg: "Failed to send API Gateway resource".to_string(),
                    })?;
                get_apis_bar.inc(1);
            }
            _ => {
                return Err(CliError {
                    msg: "Invalid resource type".to_string(),
                });
            }
        }
    }

    get_apis_bar.finish();
    Ok(())
}
