use std::io::{copy, BufReader};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use crate::commands::cloud_linter::api_gateway::process_api_gateway_resources;
use aws_config::retry::RetryConfig;
use aws_config::{BehaviorVersion, Region};
use chrono::{NaiveDate, NaiveDateTime, Utc};
use flate2::write::GzEncoder;
use flate2::Compression;
use governor::{Quota, RateLimiter};
use indicatif::ProgressBar;
use momento_cli_opts::CloudLinterResources;
use struson::writer::{JsonStreamWriter, JsonWriter};
use tokio::fs::{metadata, File};
use tokio::sync::mpsc::{self, Sender};

use crate::commands::cloud_linter::dynamodb::process_ddb_resources;
use crate::commands::cloud_linter::s3::process_s3_resources;
use crate::commands::cloud_linter::serverless_elasticache::process_serverless_elasticache_resources;
use crate::commands::cloud_linter::utils::check_aws_credentials;
use crate::error::CliError;

use super::elasticache::process_elasticache_resources;
use super::resource::Resource;

#[allow(clippy::too_many_arguments)]
pub async fn run_cloud_linter(
    region: String,
    enable_ddb_ttl_check: bool,
    enable_gsi: bool,
    enable_s3: bool,
    enable_api_gateway: bool,
    only_collect_for_resource: Option<CloudLinterResources>,
    metric_collection_rate: u32,
    start_date: Option<String>,
    end_date: Option<String>,
) -> Result<(), CliError> {
    let (tx, mut rx) = mpsc::channel::<Resource>(32);
    let file_path = "linter_results.json";
    // first we check to make sure we have perms to write files to the current directory
    check_output_is_writable(file_path).await?;

    let (metric_start_time, metric_end_time) = get_metric_time_range(start_date, end_date)?;

    // here we write the unzipped json file, containing all the linter results
    let unzipped_tokio_file = File::create(file_path).await?;
    let mut unzipped_file = unzipped_tokio_file.into_std().await;
    let mut json_writer = JsonStreamWriter::new(&mut unzipped_file);
    json_writer.begin_object()?;
    json_writer.name("resources")?;
    json_writer.begin_array()?;
    tokio::spawn(async move {
        let _ = process_data(
            region,
            tx,
            enable_ddb_ttl_check,
            enable_gsi,
            enable_s3,
            enable_api_gateway,
            only_collect_for_resource,
            metric_collection_rate,
            metric_start_time,
            metric_end_time,
        )
        .await;
    });
    while let Some(message) = rx.recv().await {
        let _ = json_writer.serialize_value(&message);
    }
    json_writer.end_array()?;
    json_writer.end_object()?;
    json_writer.finish_document()?;

    // now we compress the json into a .gz file for the customer to upload
    let compression_bar = ProgressBar::new_spinner().with_message(format!(
        "Compressing and writing to {} and {}.gz",
        file_path, file_path
    ));
    compression_bar.enable_steady_tick(Duration::from_millis(100));
    let opened_file_tokio = File::open(file_path).await?;
    let opened_file = opened_file_tokio.into_std().await;
    let mut unzipped_file = BufReader::new(opened_file);
    let zipped_file_output_tokio = File::create(format!("{}.gz", file_path)).await?;
    let zipped_file_output = zipped_file_output_tokio.into_std().await;
    let mut gz = GzEncoder::new(zipped_file_output, Compression::default());
    copy(&mut unzipped_file, &mut gz)?;
    gz.finish()?;

    compression_bar.finish();
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn process_data(
    region: String,
    sender: Sender<Resource>,
    enable_ddb_ttl_check: bool,
    enable_gsi: bool,
    enable_s3: bool,
    enable_api_gateway: bool,
    only_collect_for_resource: Option<CloudLinterResources>,
    metric_collection_rate: u32,
    metrics_start_millis: i64,
    metrics_end_millis: i64,
) -> Result<(), CliError> {
    let retry_config = RetryConfig::adaptive()
        .with_initial_backoff(Duration::from_millis(250))
        .with_max_attempts(20)
        .with_max_backoff(Duration::from_secs(5));
    let config = aws_config::defaults(BehaviorVersion::latest())
        .region(Region::new(region))
        .retry_config(retry_config)
        .load()
        .await;
    check_aws_credentials(&config).await?;

    let control_plane_quota = Quota::per_second(
        core::num::NonZeroU32::new(10).expect("should create non-zero control_plane_quota"),
    );
    let control_plane_limiter = Arc::new(RateLimiter::direct(control_plane_quota));

    let describe_ttl_quota = Quota::per_second(
        core::num::NonZeroU32::new(1).expect("should create non-zero describe_ttl_quota"),
    );
    let describe_ttl_limiter = Arc::new(RateLimiter::direct(describe_ttl_quota));

    let metrics_quota = Quota::per_second(
        core::num::NonZeroU32::new(metric_collection_rate).expect("should create non-zero quota"),
    );
    let metrics_limiter = Arc::new(RateLimiter::direct(metrics_quota));

    if let Some(resource) = only_collect_for_resource {
        return match resource {
            CloudLinterResources::ApiGateway => {
                process_api_gateway_resources(
                    &config,
                    Arc::clone(&control_plane_limiter),
                    Arc::clone(&metrics_limiter),
                    sender.clone(),
                    metrics_start_millis,
                    metrics_end_millis,
                )
                .await?;
                Ok(())
            }
            CloudLinterResources::S3 => {
                process_s3_resources(
                    &config,
                    Arc::clone(&control_plane_limiter),
                    Arc::clone(&metrics_limiter),
                    sender.clone(),
                    metrics_start_millis,
                    metrics_end_millis,
                )
                .await?;
                Ok(())
            }
            CloudLinterResources::Dynamo => {
                process_ddb_resources(
                    &config,
                    Arc::clone(&control_plane_limiter),
                    Arc::clone(&metrics_limiter),
                    Arc::clone(&describe_ttl_limiter),
                    sender.clone(),
                    metrics_start_millis,
                    metrics_end_millis,
                    enable_ddb_ttl_check,
                    enable_gsi,
                )
                .await?;
                Ok(())
            }
            CloudLinterResources::ElastiCache => {
                process_elasticache_resources(
                    &config,
                    Arc::clone(&control_plane_limiter),
                    Arc::clone(&metrics_limiter),
                    sender.clone(),
                    metrics_start_millis,
                    metrics_end_millis,
                )
                .await?;

                process_serverless_elasticache_resources(
                    &config,
                    Arc::clone(&control_plane_limiter),
                    Arc::clone(&metrics_limiter),
                    sender.clone(),
                    metrics_start_millis,
                    metrics_end_millis,
                )
                .await?;
                Ok(())
            }
        };
    };

    if enable_s3 {
        process_s3_resources(
            &config,
            Arc::clone(&control_plane_limiter),
            Arc::clone(&metrics_limiter),
            sender.clone(),
            metrics_start_millis,
            metrics_end_millis,
        )
        .await?;
    }

    if enable_api_gateway {
        process_api_gateway_resources(
            &config,
            Arc::clone(&control_plane_limiter),
            Arc::clone(&metrics_limiter),
            sender.clone(),
            metrics_start_millis,
            metrics_end_millis,
        )
        .await?;
    }

    process_ddb_resources(
        &config,
        Arc::clone(&control_plane_limiter),
        Arc::clone(&metrics_limiter),
        Arc::clone(&describe_ttl_limiter),
        sender.clone(),
        metrics_start_millis,
        metrics_end_millis,
        enable_ddb_ttl_check,
        enable_gsi,
    )
    .await?;

    process_elasticache_resources(
        &config,
        Arc::clone(&control_plane_limiter),
        Arc::clone(&metrics_limiter),
        sender.clone(),
        metrics_start_millis,
        metrics_end_millis,
    )
    .await?;

    process_serverless_elasticache_resources(
        &config,
        Arc::clone(&control_plane_limiter),
        Arc::clone(&metrics_limiter),
        sender.clone(),
        metrics_start_millis,
        metrics_end_millis,
    )
    .await?;

    Ok(())
}

/// Calculates a metric time range based on optional start and end dates.
/// If start_date is not provided, it defaults to end_date - 30 days.
/// If end_date is not provided, it defaults to now.
///
/// # Arguments
///
/// * `start_date` - An optional String representing the start date in "YYYY-MM-DD" format.
/// * `end_date` - An optional String representing the end date in "YYYY-MM-DD" format.
///
/// # Returns
///
/// A Result containing a tuple of the start and end timestamps in millis, or a CliError
/// if date parsing fails.
fn get_metric_time_range(
    start_date: Option<String>,
    end_date: Option<String>,
) -> Result<(i64, i64), CliError> {
    let now = Utc::now().naive_utc();
    let thirty_days = chrono::Duration::days(30).num_milliseconds();

    let processed_end_date = end_date
        .map(|date| parse_date_string(&date))
        .transpose()?
        .unwrap_or(now)
        .timestamp_millis();
    let processed_start_date = start_date
        .map(|date| parse_date_string(&date))
        .transpose()?
        .map(|date| date.timestamp_millis())
        .unwrap_or_else(|| processed_end_date - thirty_days);

    Ok((processed_start_date, processed_end_date))
}

fn parse_date_string(date: &str) -> Result<NaiveDateTime, CliError> {
    let naive_date = NaiveDate::parse_from_str(date, "%Y-%m-%d").map_err(|_| CliError {
        msg: "Date must be in ISO 8601 (YYYY-MM-DD) format".to_string(),
    })?;
    naive_date.and_hms_opt(0, 0, 0).ok_or_else(|| CliError {
        msg: "invalid time".to_string(),
    })
}

async fn check_output_is_writable(file_path: &str) -> Result<(), CliError> {
    let path = Path::new(file_path);

    // Get the parent of the output file path
    let dir = if path.is_absolute() {
        path.parent().unwrap_or(path)
    } else {
        Path::new(".")
    };

    let metadata = metadata(dir).await.map_err(|_| CliError {
        msg: format!("Directory '{}' is not accessible", dir.display()),
    })?;

    if metadata.permissions().readonly() {
        Err(CliError {
            msg: format!("Directory '{}' is not writable", dir.display()),
        })
    } else {
        Ok(())
    }
}
