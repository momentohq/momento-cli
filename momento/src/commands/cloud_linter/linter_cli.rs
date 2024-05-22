use std::io::{copy, BufReader};
use std::path::Path;
use std::sync::Arc;

use aws_config::{BehaviorVersion, Region};
use flate2::write::GzEncoder;
use flate2::Compression;
use governor::{Quota, RateLimiter};
use struson::writer::{JsonStreamWriter, JsonWriter};
use tokio::fs::{metadata, File};
use tokio::sync::mpsc::{self, Sender};
use crate::commands::cloud_linter::api_gateway::process_api_gateway_resources;

use crate::commands::cloud_linter::dynamodb::process_ddb_resources;
use crate::commands::cloud_linter::s3::process_s3_resources;
use crate::commands::cloud_linter::serverless_elasticache::process_serverless_elasticache_resources;
use crate::commands::cloud_linter::utils::check_aws_credentials;
use crate::error::CliError;

use super::elasticache::process_elasticache_resources;
use super::resource::Resource;

pub async fn run_cloud_linter(region: String) -> Result<(), CliError> {
    let (tx, mut rx) = mpsc::channel::<Resource>(32);
    let file_path = "linter_results.json";
    // first we check to make sure we have perms to write files to the current directory
    check_output_is_writable(file_path).await?;

    // here we write the unzipped json file, containing all the linter results
    let unzipped_tokio_file = File::create(file_path).await?;
    let mut unzipped_file = unzipped_tokio_file.into_std().await;
    let mut json_writer = JsonStreamWriter::new(&mut unzipped_file);
    json_writer.begin_object()?;
    json_writer.name("resources")?;
    json_writer.begin_array()?;
    tokio::spawn(async move {
        let _ = process_data(region, tx).await;
    });
    while let Some(message) = rx.recv().await {
        let _ = json_writer.serialize_value(&message);
    }
    json_writer.end_array()?;
    json_writer.end_object()?;
    json_writer.finish_document()?;

    // now we compress the json into a .gz file for the customer to upload
    let opened_file_tokio = File::open(file_path).await?;
    let opened_file = opened_file_tokio.into_std().await;
    let mut unzipped_file = BufReader::new(opened_file);
    let zipped_file_output_tokio = File::create("linter_results.json.gz").await?;
    let zipped_file_output = zipped_file_output_tokio.into_std().await;
    let mut gz = GzEncoder::new(zipped_file_output, Compression::default());
    copy(&mut unzipped_file, &mut gz)?;
    gz.finish()?;

    Ok(())
}

async fn process_data(region: String, sender: Sender<Resource>) -> Result<(), CliError> {
    let config = aws_config::defaults(BehaviorVersion::latest())
        .region(Region::new(region))
        .load()
        .await;
    check_aws_credentials(&config).await?;

    let control_plane_quota = Quota::per_second(
        core::num::NonZeroU32::new(10).expect("should create non-zero control_plane_quota"),
    );
    let control_plane_limiter = Arc::new(RateLimiter::direct(control_plane_quota));

    let describe_ttl_quota = Quota::per_second(
        core::num::NonZeroU32::new(3).expect("should create non-zero describe_ttl_quota"),
    );
    let describe_ttl_limiter = Arc::new(RateLimiter::direct(describe_ttl_quota));

    let metrics_quota =
        Quota::per_second(core::num::NonZeroU32::new(20).expect("should create non-zero quota"));
    let metrics_limiter = Arc::new(RateLimiter::direct(metrics_quota));

    process_s3_resources(
        &config,
        Arc::clone(&metrics_limiter),
        Arc::clone(&control_plane_limiter),
        sender.clone(),
    )
    .await?;

    process_api_gateway_resources(&config, Arc::clone(&metrics_limiter), sender.clone()).await?;

    process_ddb_resources(
        &config,
        Arc::clone(&control_plane_limiter),
        Arc::clone(&metrics_limiter),
        Arc::clone(&describe_ttl_limiter),
        sender.clone(),
    )
    .await?;

    process_elasticache_resources(
        &config,
        Arc::clone(&control_plane_limiter),
        Arc::clone(&metrics_limiter),
        sender.clone(),
    )
    .await?;

    process_serverless_elasticache_resources(
        &config,
        Arc::clone(&control_plane_limiter),
        Arc::clone(&metrics_limiter),
        sender.clone(),
    )
    .await?;

    Ok(())
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
