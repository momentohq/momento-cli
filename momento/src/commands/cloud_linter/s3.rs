use std::collections::HashMap;
use std::sync::Arc;
use aws_config::SdkConfig;
use aws_sdk_s3::types::Object;
use governor::DefaultDirectRateLimiter;
use phf::{Map, phf_map};
use serde::Serialize;
use tokio::sync::mpsc::Sender;
use crate::commands::cloud_linter::metrics::{AppendMetrics, Metric, MetricTarget, ResourceWithMetrics};
use crate::commands::cloud_linter::resource::{Resource, ResourceType, S3Resource};
use crate::commands::cloud_linter::utils::rate_limit;
use crate::error::CliError;

const S3_METRICS_STANDARD_STORAGE_TYPE: Map<&'static str, &'static [&'static str]> = phf_map! {
    "Sum" => &[
        "BucketSizeBytes",
        // "BytesDownloaded",
    ],
    "Average" => &[
        "BucketSizeBytes",
        // "BytesDownloaded",
    ],
    "Maximum" => &[
        "BucketSizeBytes",
        // "BytesDownloaded",
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


#[derive(Serialize, Clone, Debug)]
pub(crate) struct S3Metadata {
    #[serde(rename = "fakeStat")]
    fake_stat: i64,
    // #[serde(rename = "objectCount")]
    // object_count: i64,
    // #[serde(rename = "size")]
    // size: Option<i64>,
    // #[serde(rename = "storageClass")]
    // storage_class: String
}

impl ResourceWithMetrics for S3Resource {
    fn create_metric_targets(&self) -> Result<Vec<MetricTarget>, CliError> {
        match self.resource_type {
            ResourceType::S3 => {
                Ok(vec![
                    MetricTarget {
                        namespace: "AWS/S3".to_string(),
                        dimensions: HashMap::from([
                            ("BucketName".to_string(), self.id.clone()),
                            // TODO: think about processing storage type in some way
                            ("StorageType".to_string(), "StandardStorage".to_string())
                        ]),
                        targets: S3_METRICS_STANDARD_STORAGE_TYPE,
                    },
                    MetricTarget {
                        namespace: "AWS/S3".to_string(),
                        dimensions: HashMap::from([
                            ("BucketName".to_string(), self.id.clone()),
                            ("StorageType".to_string(), "AllStorageTypes".to_string())
                        ]),
                        targets: S3_METRICS_ALL_STORAGE_TYPES,
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
    sender: Sender<Resource>,
) -> Result<(), CliError> {
    println!("Processing S3 resources");
    let region = config.region().map(|r| r.as_ref()).ok_or(CliError {
        msg: "No region configured for client".to_string(),
    })?;
    let s3client = aws_sdk_s3::Client::new(config);
    let metrics_client = aws_sdk_cloudwatch::Client::new(config);

    let bucket_names = list_buckets(&s3client).await?;
    process_buckets(bucket_names, region, sender, &metrics_client, &metrics_limiter).await?;

    // println!("Listing directory buckets");
    // let bucket_names = list_directory_buckets(&s3client).await?;

    Ok(())
}

async fn process_buckets(
    buckets: Vec<String>,
    region: &str,
    sender: Sender<Resource>,
    metrics_client: &aws_sdk_cloudwatch::Client,
    metrics_limiter: &Arc<DefaultDirectRateLimiter>
) -> Result<(), CliError> {
    println!("Processing S3 Buckets in region: {}", region);
    let mut resources: Vec<Resource> = Vec::new();

    for bucket in buckets {
        let metadata = S3Metadata {
            fake_stat: 42,
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
    }


    for mut resource in resources {
        match resource {
            Resource::S3(mut my_resource) => {
                my_resource.append_metrics(
                    &metrics_client,
                    Arc::clone(&metrics_limiter)
                ).await?;
                sender
                    .send(Resource::S3(my_resource))
                    .await
                    .map_err(|_| CliError {
                        msg: "Failed to send S3 resource".to_string(),
                    })?;
            }
            _ => {
                return Err(CliError {
                    msg: "Invalid resource type".to_string()
                });
            }
        }
    }
    Ok(())
}

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
        println!("Directory Bucket: {}", bucket.name().unwrap_or_default());
    }
    Ok(bucket_names)
}


async fn list_objects_in_bucket(
    s3_client: &aws_sdk_s3::Client,
    bucket_name: &str,
    limiter: Arc<DefaultDirectRateLimiter>,
) -> Result<Vec<Object>, CliError>{
    let mut s3_objects = Vec::new();
    let mut obj_stream =
        s3_client
            .list_objects_v2()
            .bucket(bucket_name)
            .into_paginator()
            .send();
    while let Some(result) = rate_limit(Arc::clone(&limiter), || obj_stream.next()).await {
        match result {
            Ok(result) => {
                if let Some(objects) = result.contents {
                    for obj in objects {
                        s3_objects.push(obj);
                    }
                }
            }
            Err(err) => {
                println!("Failed to list objects in bucket: {}", err);
            }
        }
    }
    Ok(s3_objects)
}
