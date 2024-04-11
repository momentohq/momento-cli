use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use aws_config::{BehaviorVersion, Region};
use flate2::write::GzEncoder;
use flate2::Compression;
use governor::{Quota, RateLimiter};
use indicatif::ProgressBar;
use tokio::fs::{metadata, File};
use tokio::io::AsyncWriteExt;

use crate::commands::cloud_linter::dynamodb::get_ddb_resources;
use crate::commands::cloud_linter::elasticache::get_elasticache_resources;
use crate::commands::cloud_linter::metrics::append_metrics_to_resources;
use crate::commands::cloud_linter::resource::DataFormat;
use crate::commands::cloud_linter::utils::check_aws_credentials;
use crate::error::CliError;

pub async fn run_cloud_linter(region: String) -> Result<(), CliError> {
    let config = aws_config::defaults(BehaviorVersion::latest())
        .region(Region::new(region))
        .load()
        .await;
    check_aws_credentials(&config).await?;

    let output_file_path = "linter_results.json.gz";
    check_output_is_writable(output_file_path).await?;

    let fast_quota = Quota::per_second(
        core::num::NonZeroU32::new(10).expect("should create non-zero fast_quota"),
    );
    let fast_limiter = Arc::new(RateLimiter::direct(fast_quota));

    let slow_quota = Quota::per_second(
        core::num::NonZeroU32::new(1).expect("should create non-zero slow_quota"),
    );
    let slow_limiter = Arc::new(RateLimiter::direct(slow_quota));

    let mut resources = get_ddb_resources(&config, Arc::clone(&slow_limiter)).await?;

    let mut elasticache_resources =
        get_elasticache_resources(&config, Arc::clone(&fast_limiter)).await?;
    resources.append(&mut elasticache_resources);

    let resources =
        append_metrics_to_resources(&config, Arc::clone(&fast_limiter), resources).await?;

    let data_format = DataFormat { resources };

    write_data_to_file(data_format, output_file_path).await?;

    Ok(())
}

async fn write_data_to_file(data_format: DataFormat, file_path: &str) -> Result<(), CliError> {
    let bar = ProgressBar::new_spinner().with_message("Writing data to file");
    bar.enable_steady_tick(Duration::from_millis(100));

    let data_format_json = serde_json::to_string(&data_format)?;

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data_format_json.as_bytes())?;

    let compressed_json = encoder.finish()?;

    let mut file = File::create(file_path).await?;
    file.write_all(&compressed_json).await?;

    bar.finish();

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
