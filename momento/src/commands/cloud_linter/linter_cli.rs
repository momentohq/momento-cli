use std::sync::Arc;

use aws_config::{BehaviorVersion, Region};
use governor::{Quota, RateLimiter};

use crate::commands::cloud_linter::dynamodb::get_ddb_metadata;
use crate::commands::cloud_linter::elasticache::get_elasticache_metadata;
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

    let ddb_metadata = get_ddb_metadata(&config, Arc::clone(&limiter)).await?;

    let ddb_json = serde_json::to_string_pretty(&ddb_metadata)?;
    console_info!("DynamoDB metadata:\n{}", ddb_json);

    let elasticache_metadata = get_elasticache_metadata(&config, Arc::clone(&limiter)).await?;
    let elasticache_json = serde_json::to_string_pretty(&elasticache_metadata)?;
    console_info!("ElastiCache metadata:\n{}", elasticache_json);

    Ok(())
}
