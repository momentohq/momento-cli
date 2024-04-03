use std::sync::Arc;

use aws_config::{BehaviorVersion, Region};
use governor::{Quota, RateLimiter};

use crate::commands::cloud_linter::dynamodb::get_ddb_resources;
use crate::commands::cloud_linter::elasticache::get_elasticache_resources;
use crate::commands::cloud_linter::metrics::append_metrics_to_resources;
use crate::commands::cloud_linter::resource::DataFormat;
use crate::error::CliError;
use crate::utils::console::console_info;

pub async fn run_cloud_linter(region: String) -> Result<(), CliError> {
    let config = aws_config::defaults(BehaviorVersion::latest())
        .region(Region::new(region))
        .load()
        .await;

    let quota =
        Quota::per_second(core::num::NonZeroU32::new(1).expect("should create non-zero quota"));
    let limiter = Arc::new(RateLimiter::direct(quota));

    let mut resources = get_ddb_resources(&config, Arc::clone(&limiter)).await?;

    let mut elasticache_resources =
        get_elasticache_resources(&config, Arc::clone(&limiter)).await?;
    resources.append(&mut elasticache_resources);

    let resources = append_metrics_to_resources(&config, Arc::clone(&limiter), resources).await?;

    let data_format = DataFormat { resources };
    let data_format_json = serde_json::to_string_pretty(&data_format)?;

    console_info!("{}", data_format_json);

    Ok(())
}
