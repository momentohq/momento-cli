use std::collections::HashMap;
use std::sync::Arc;
use aws_config::SdkConfig;
// use aws_sdk_s3::Client;
// use aws_sdk_s3::operation::get_bucket_encryption::GetBucketEncryptionOutput;
// use aws_sdk_s3::types::ServerSideEncryptionConfiguration;
use governor::DefaultDirectRateLimiter;
use indicatif::{ProgressBar, ProgressStyle};
use phf::{Map, phf_map};
use serde::Serialize;
use tokio::sync::mpsc::Sender;
use crate::commands::cloud_linter::metrics::{AppendMetrics, Metric, MetricTarget, ResourceWithMetrics};
use crate::commands::cloud_linter::resource::{Resource, ResourceType, S3Resource};
// use crate::commands::cloud_linter::utils::rate_limit;
use crate::error::CliError;

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

// TODO: Do we need each calc type for each metric? Can we trim some of these?
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
        "FirstByteLatency",
        "TotalRequestLatency",
        // These metrics cause the program to exit prematurely without error
        // "4xxErrors",
        // "5xxErrors",
    ],
    "Average" => &[
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
        "FirstByteLatency",
        "TotalRequestLatency",
        // These metrics cause the program to exit prematurely without error
        // "4xxErrors",
        // "5xxErrors",
    ],
    "Maximum" => &[
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
        "FirstByteLatency",
        "TotalRequestLatency",
        // These metrics cause the program to exit prematurely without error
        // "4xxErrors",
        // "5xxErrors",
    ],
};


#[derive(Serialize, Clone, Debug)]
pub(crate) struct S3Metadata {
    #[serde(rename = "bucketType")]
    bucket_type: String,
}

fn get_metric_target_for_storage_type(name: &str, storage_type: &str) -> MetricTarget {
    MetricTarget {
        prefix: format!("{}_", storage_type.to_lowercase().to_string()),
        namespace: "AWS/S3".to_string(),
        dimensions: HashMap::from([
            ("BucketName".to_string(), name.to_string()),
            ("StorageType".to_string(), storage_type.to_string())
        ]),
        targets: S3_METRICS_STANDARD_STORAGE_TYPES,
    }
}

fn get_storage_types() -> Vec<&'static str> {
    vec![
        "StandardStorage",
        "IntelligentTieringAAStorage",
        "IntelligentTieringAIAStorage",
        "IntelligentTieringDAAStorage",
        "IntelligentTieringFAStorage",
        "IntelligentTieringIAStorage",
        "StandardIAStorage",
        "StandardIASizeOverhead",
        "IntAAObjectOverhead",
        "IntAAS3ObjectOverhead",
        "IntDAAObjectOverhead",
        "IntDAAS3ObjectOverhead",
        "OneZoneIAStorage",
        "OneZoneIASizeOverhead",
        "ReducedRedundancyStorage",
        "GlacierInstantRetrievalSizeOverhead",
        "GlacierInstantRetrievalStorage",
        "GlacierStorage",
        "GlacierStagingStorage",
        "GlacierObjectOverhead",
        "GlacierS3ObjectOverhead",
        "DeepArchiveStorage",
        "DeepArchiveObjectOverhead",
        "DeepArchiveS3ObjectOverhead",
        "DeepArchiveStagingStorage",
        "ExpressOneZone",
    ]
}

impl ResourceWithMetrics for S3Resource {
    fn create_metric_targets(&self) -> Result<Vec<MetricTarget>, CliError> {
        let storage_types = get_storage_types();
        match self.resource_type {
            ResourceType::S3 => {
                Ok(vec![
                    get_metric_target_for_storage_type(&self.id, storage_types[0]),
                    get_metric_target_for_storage_type(&self.id, storage_types[1]),
                    get_metric_target_for_storage_type(&self.id, storage_types[2]),
                    get_metric_target_for_storage_type(&self.id, storage_types[3]),
                    get_metric_target_for_storage_type(&self.id, storage_types[4]),
                    get_metric_target_for_storage_type(&self.id, storage_types[5]),
                    get_metric_target_for_storage_type(&self.id, storage_types[6]),
                    get_metric_target_for_storage_type(&self.id, storage_types[7]),
                    get_metric_target_for_storage_type(&self.id, storage_types[8]),
                    get_metric_target_for_storage_type(&self.id, storage_types[9]),
                    get_metric_target_for_storage_type(&self.id, storage_types[10]),
                    get_metric_target_for_storage_type(&self.id, storage_types[11]),
                    get_metric_target_for_storage_type(&self.id, storage_types[12]),
                    get_metric_target_for_storage_type(&self.id, storage_types[13]),
                    get_metric_target_for_storage_type(&self.id, storage_types[14]),
                    get_metric_target_for_storage_type(&self.id, storage_types[15]),
                    get_metric_target_for_storage_type(&self.id, storage_types[16]),
                    get_metric_target_for_storage_type(&self.id, storage_types[17]),
                    get_metric_target_for_storage_type(&self.id, storage_types[18]),
                    get_metric_target_for_storage_type(&self.id, storage_types[19]),
                    get_metric_target_for_storage_type(&self.id, storage_types[20]),
                    get_metric_target_for_storage_type(&self.id, storage_types[21]),
                    get_metric_target_for_storage_type(&self.id, storage_types[22]),
                    get_metric_target_for_storage_type(&self.id, storage_types[23]),
                    get_metric_target_for_storage_type(&self.id, storage_types[24]),
                    get_metric_target_for_storage_type(&self.id, storage_types[25]),
                    MetricTarget {
                        prefix: "".to_string(),
                        namespace: "AWS/S3".to_string(),
                        dimensions: HashMap::from([
                            ("BucketName".to_string(), self.id.clone()),
                            ("StorageType".to_string(), "AllStorageTypes".to_string())
                        ]),
                        targets: S3_METRICS_ALL_STORAGE_TYPES,
                    },
                    MetricTarget {
                        prefix: "".to_string(),
                        namespace: "AWS/S3".to_string(),
                        dimensions: HashMap::from([
                            ("BucketName".to_string(), self.id.clone()),
                            // TODO: a filter is required to get these metrics. Can we either
                            //  require a filter with a specific id or require elevated privs
                            //  that allow us to create a filter on the buckets for them?
                            //  OR, do we just want to skip these metrics.
                            ("FilterId".to_string(), "all-objects".to_string())
                        ]),
                        targets: S3_METRICS_REQUEST,
                    },
                ])
            }
            _ => Err(CliError {
                msg: "Invalid resource type".to_string()
            })
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
    _control_plane_limiter: Arc<DefaultDirectRateLimiter>,
    metrics_limiter: Arc<DefaultDirectRateLimiter>,
    // bucket_encryption_limiter: Arc<DefaultDirectRateLimiter>,
    sender: Sender<Resource>,
) -> Result<(), CliError> {
    println!("Processing S3 resources");
    let region = config.region().map(|r| r.as_ref()).ok_or(CliError {
        msg: "No region configured for client".to_string(),
    })?;
    let s3client = aws_sdk_s3::Client::new(config);
    let metrics_client = aws_sdk_cloudwatch::Client::new(config);

    let list_buckets_bar = ProgressBar::new_spinner().with_message("Listing S3 General Purpose Buckets");
    list_buckets_bar.enable_steady_tick(std::time::Duration::from_millis(100));
    let bucket_names = list_buckets(&s3client).await?;
    list_buckets_bar.finish();

    process_buckets(
        // s3client.clone(),
        bucket_names,
        "general_purpose",
        region,
        sender.clone(),
        &metrics_client,
        &metrics_limiter,
        // &bucket_encryption_limiter,
    ).await?;

    let list_buckets_bar = ProgressBar::new_spinner().with_message("Listing S3 Directory Buckets");
    list_buckets_bar.enable_steady_tick(std::time::Duration::from_millis(100));
    let bucket_names = list_directory_buckets(&s3client).await?;
    list_buckets_bar.finish();

    process_buckets(
        // s3client.clone(),
        bucket_names,
        "directory",
        region,
        sender,
        &metrics_client,
        &metrics_limiter,
        // &bucket_encryption_limiter,
    ).await?;

    Ok(())
}

async fn process_buckets(
    // s3client: Client,
    buckets: Vec<String>,
    bucket_type: &str,
    region: &str,
    sender: Sender<Resource>,
    metrics_client: &aws_sdk_cloudwatch::Client,
    metrics_limiter: &Arc<DefaultDirectRateLimiter>,
    // encryption_limiter: &Arc<DefaultDirectRateLimiter>,
) -> Result<(), CliError> {
    let mut resources: Vec<Resource> = Vec::new();

    let process_buckets_bar = ProgressBar::new((buckets.len() * 2) as u64)
        .with_message("Processing S3 Buckets");
    process_buckets_bar.set_style(ProgressStyle::with_template("  {msg} {bar} {eta}").expect("invalid template"));
    for bucket in buckets {
        let metadata = S3Metadata {
            bucket_type: bucket_type.to_string(),
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
                my_resource.append_metrics(
                    &metrics_client,
                    Arc::clone(&metrics_limiter)
                ).await?;
                // if bucket_type != "directory" {
                //     let bucket_encryption = get_bucket_encryption(s3client.clone(), &my_resource.id, encryption_limiter).await?;
                //     // my_resource.metrics.bucket_encryption = bucket_encryption;
                // }
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
                    msg: "Invalid resource type".to_string()
                });
            }
        }
    }
    process_buckets_bar.finish();
    Ok(())
}

// async fn get_bucket_encryption(
//     s3_client: Client,
//     bucket_name: &str,
//     limiter: &Arc<DefaultDirectRateLimiter>,
// ) -> Result<(), CliError> {
//     let encryption: GetBucketEncryptionOutput = rate_limit(Arc::clone(&limiter), || async {
//         s3_client
//             .get_bucket_encryption()
//             .bucket(bucket_name)
//             .send()
//             .await
//             .expect("Failed getting bucket encryption");
//     })
//     .await;
//     let encryption_output = matches!(
//         encryption.server_side_encryption_configuration,
//         Some(GetBucketEncryptionOutput {
//             server_side_encryption_configuration: Some(_),
//             ..
//         })
//     );
//     println!("Bucket Encryption: {:?}", encryption.server_side_encryption_configuration);
//     Ok(())
// }

async fn list_buckets(
    s3_client: &aws_sdk_s3::Client
) -> Result<Vec<String>, CliError> {
    let mut bucket_names = Vec::new();
    let resp = s3_client.list_buckets().send().await?;
    let buckets = resp.buckets();
    for bucket in buckets {
        bucket_names.push(bucket.name().unwrap_or_default().to_string());
    }
    Ok(bucket_names)
}

async fn list_directory_buckets(
    s3_client: &aws_sdk_s3::Client,
) -> Result<Vec<String>, CliError> {
    let mut bucket_names = Vec::new();
    let resp = s3_client.list_directory_buckets().send().await?;
    let buckets = resp.buckets();
    for bucket in buckets {
        bucket_names.push(bucket.name().unwrap_or_default().to_string());
    }
    Ok(bucket_names)
}
