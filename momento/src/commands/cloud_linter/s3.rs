use crate::commands::cloud_linter::metrics::{
    AppendMetrics, Metric, MetricTarget, ResourceWithMetrics,
};
use crate::commands::cloud_linter::resource::{Resource, ResourceType, S3Resource};
use crate::commands::cloud_linter::utils::rate_limit;
use crate::error::CliError;
use aws_config::SdkConfig;
use aws_sdk_s3::error::ProvideErrorMetadata;
use aws_sdk_s3::types::MetricsConfiguration;
use governor::DefaultDirectRateLimiter;
use indicatif::{ProgressBar, ProgressStyle};
use phf::{phf_map, Map};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

const S3_METRICS_STANDARD_STORAGE_TYPES: Map<&'static str, &'static [&'static str]> = phf_map! {
    "Sum" => &[
        "BucketSizeBytes",
    ],
    "Average" => &[
        "BucketSizeBytes",
    ],
    "Maximum" => &[
        "BucketSizeBytes",
    ],
};

const S3_METRICS_ALL_STORAGE_TYPES: Map<&'static str, &'static [&'static str]> = phf_map! {
    "Sum" => &[
        "NumberOfObjects",
    ],
    "Average" => &[
        "NumberOfObjects",
    ],
    "Maximum" => &[
        "NumberOfObjects",
    ],
};

const S3_METRICS_REQUEST: Map<&'static str, &'static [&'static str]> = phf_map! {
    "Sum" => &[
        "AllRequests",
        "GetRequests",
        "PutRequests",
        "DeleteRequests",
        "HeadRequests",
        "PostRequests",
        "SelectRequests",
        "SelectBytesScanned",
        "SelectBytesReturned",
        "ListRequests",
        "BytesDownloaded",
        "BytesUploaded",
    ],
    "Average" => &[
        "FirstByteLatency",
        "TotalRequestLatency",
    ],
    "Maximum" => &[
        "FirstByteLatency",
        "TotalRequestLatency",
    ],
};

#[derive(Serialize, Clone, Debug)]
pub(crate) struct S3Metadata {
    #[serde(rename = "requestMetricsFilter")]
    request_metrics_filter: String,
}

impl ResourceWithMetrics for S3Resource {
    fn create_metric_targets(&self) -> Result<Vec<MetricTarget>, CliError> {
        let mut s3_metrics_targets: Vec<MetricTarget> = Vec::new();
        s3_metrics_targets.push(MetricTarget {
            namespace: "AWS/S3".to_string(),
            expression: format!("{{AWS/S3,BucketName,StorageType}} MetricName=\"BucketSizeBytes\" BucketName=\"{}\"", self.id),
            dimensions: HashMap::from([]),
            targets: S3_METRICS_STANDARD_STORAGE_TYPES,
        });
        s3_metrics_targets.push(MetricTarget {
            namespace: "AWS/S3".to_string(),
            expression: "".to_string(),
            dimensions: HashMap::from([
                ("BucketName".to_string(), self.id.clone()),
                ("StorageType".to_string(), "AllStorageTypes".to_string()),
            ]),
            targets: S3_METRICS_ALL_STORAGE_TYPES,
        });
        // If and only if the bucket has an appropriate metrics filter including all
        // objects, add the request metrics to the list of metrics to be collected.
        if !self.metadata.request_metrics_filter.is_empty() {
            let request_metrics_dimensions = HashMap::from([
                ("BucketName".to_string(), self.id.clone()),
                (
                    "FilterId".to_string(),
                    self.metadata.request_metrics_filter.to_string(),
                ),
            ]);
            s3_metrics_targets.push(MetricTarget {
                namespace: "AWS/S3".to_string(),
                expression: "".to_string(),
                dimensions: request_metrics_dimensions,
                targets: S3_METRICS_REQUEST,
            });
        }

        match self.resource_type {
            ResourceType::S3 => Ok(s3_metrics_targets),
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

pub(crate) async fn process_s3_resources(
    config: &SdkConfig,
    metrics_limiter: Arc<DefaultDirectRateLimiter>,
    control_plane_limiter: Arc<DefaultDirectRateLimiter>,
    sender: Sender<Resource>,
) -> Result<(), CliError> {
    let region = config.region().map(|r| r.as_ref()).ok_or(CliError {
        msg: "No region configured for client".to_string(),
    })?;
    let s3client = aws_sdk_s3::Client::new(config);
    let metrics_client = aws_sdk_cloudwatch::Client::new(config);

    let list_buckets_bar = ProgressBar::new_spinner().with_message("Listing S3 Buckets");
    list_buckets_bar.enable_steady_tick(std::time::Duration::from_millis(100));
    let bucket_names = list_buckets(&s3client).await.unwrap_or_else(|err| {
        eprint!("{}", err);
        vec![]
    });
    list_buckets_bar.finish();

    process_buckets(
        s3client.clone(),
        bucket_names,
        region,
        sender.clone(),
        &metrics_client,
        metrics_limiter.clone(),
        control_plane_limiter.clone(),
    )
    .await?;

    Ok(())
}

async fn list_bucket_metrics_configs(
    s3client: aws_sdk_s3::Client,
    bucket: String,
    control_plane_limiter: Arc<DefaultDirectRateLimiter>,
) -> Result<Vec<MetricsConfiguration>, CliError> {
    let mut all_configs: Vec<MetricsConfiguration> = Vec::new();
    let mut continuation_token: Option<String> = None;
    loop {
        let configs = rate_limit(Arc::clone(&control_plane_limiter), || async {
            s3client
                .list_bucket_metrics_configurations()
                .bucket(&bucket)
                .continuation_token(continuation_token.unwrap_or_default())
                .send()
                .await
        })
        .await;
        match configs {
            Ok(configs) => {
                if configs.metrics_configuration_list.is_none() {
                    break;
                }
                let metrics_configs: Vec<MetricsConfiguration> =
                    configs.metrics_configuration_list.unwrap_or_default();
                all_configs.extend(metrics_configs);
                if configs.is_truncated.unwrap_or_default() {
                    continuation_token = configs.next_continuation_token;
                } else {
                    break;
                }
            }
            Err(err) => {
                if err.code() == Some("PermanentRedirect") {
                    // https://github.com/awslabs/aws-sdk-rust/issues/183
                    // There may be some extra processing we can do to follow the redirect we're getting
                    // here, but for now we'll just print an error.
                    log::debug!("skipping redirected bucket {}", bucket);
                    break;
                }
                return Err(CliError {
                    msg: format!("Failed to get bucket metrics configuration: {}", err),
                });
            }
        }
    }
    Ok(all_configs)
}

async fn try_get_bucket_metrics_filter(
    s3client: aws_sdk_s3::Client,
    bucket: String,
    control_plane_limiter: Arc<DefaultDirectRateLimiter>,
) -> Result<String, CliError> {
    let bucket_metrics = list_bucket_metrics_configs(
        s3client.clone(),
        bucket.clone(),
        Arc::clone(&control_plane_limiter),
    )
    .await;
    match bucket_metrics {
        Ok(bucket_metrics) => {
            for config in bucket_metrics {
                // A filter value of None means all objects are included in the metrics.
                if config.filter.is_none() {
                    return Ok(config.id);
                }
            }
        }
        Err(err) => {
            return Err(CliError {
                msg: format!("{}", err),
            });
        }
    }
    Ok("".to_string())
}

async fn process_buckets(
    s3client: aws_sdk_s3::Client,
    buckets: Vec<String>,
    region: &str,
    sender: Sender<Resource>,
    metrics_client: &aws_sdk_cloudwatch::Client,
    metrics_limiter: Arc<DefaultDirectRateLimiter>,
    control_plane_limiter: Arc<DefaultDirectRateLimiter>,
) -> Result<(), CliError> {
    let mut resources: Vec<Resource> = Vec::new();

    let process_buckets_bar =
        ProgressBar::new((buckets.len() * 2) as u64).with_message("Processing S3 Buckets");
    process_buckets_bar
        .set_style(ProgressStyle::with_template("  {msg} {bar} {eta}").expect("invalid template"));
    for bucket in buckets {
        let filter_id = try_get_bucket_metrics_filter(
            s3client.clone(),
            bucket.clone(),
            Arc::clone(&control_plane_limiter),
        )
        .await;
        let filter_id = match filter_id {
            Ok(filter_id) => filter_id,
            Err(err) => {
                eprint!("{}", err);
                continue;
            }
        };

        let metadata = S3Metadata {
            request_metrics_filter: filter_id,
        };

        let s3_resource = S3Resource {
            resource_type: ResourceType::S3,
            region: region.to_string(),
            id: bucket.clone(),
            metrics: vec![],
            metric_period_seconds: 0,
            metadata,
        };
        resources.push(Resource::S3(s3_resource));
        process_buckets_bar.inc(1);
    }

    for resource in resources {
        match resource {
            Resource::S3(mut my_resource) => {
                my_resource
                    .append_metrics(metrics_client, Arc::clone(&metrics_limiter))
                    .await?;
                sender
                    .send(Resource::S3(my_resource))
                    .await
                    .map_err(|_| CliError {
                        msg: "Failed to send S3 resource".to_string(),
                    })?;
                process_buckets_bar.inc(1);
            }
            _ => {
                return Err(CliError {
                    msg: "Invalid resource type".to_string(),
                });
            }
        }
    }
    process_buckets_bar.finish();
    Ok(())
}

async fn list_buckets(s3_client: &aws_sdk_s3::Client) -> Result<Vec<String>, CliError> {
    let mut bucket_names = Vec::new();
    let resp = s3_client.list_buckets().send().await?;
    let buckets = resp.buckets();
    for bucket in buckets {
        bucket_names.push(bucket.name().unwrap_or_default().to_string());
    }
    Ok(bucket_names)
}
