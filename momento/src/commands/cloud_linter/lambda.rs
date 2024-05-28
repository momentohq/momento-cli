use std::sync::Arc;
use aws_config::SdkConfig;
use governor::DefaultDirectRateLimiter;
use phf::{Map, phf_map};
use serde::Serialize;
use tokio::sync::mpsc::Sender;
use crate::commands::cloud_linter::resource::Resource;
use crate::error::CliError;

const LAMBDA_METRICS: Map<&'static str, &'static [&'static str]> = phf_map! {
    "Sum" => &[
    ],
    "Average" => &[
    ],
    "Maximum" => &[
    ],
};

#[derive(Serialize, Clone, Debug)]
pub(crate) struct LambdaMetadata {
    #[serde(rename = "aString")]
    a_string: String,
}

pub(crate) async fn process_s3_resources(
    config: &SdkConfig,
    metrics_limiter: Arc<DefaultDirectRateLimiter>,
    sender: Sender<Resource>,
) -> Result<(), CliError> {
    let region = config.region().map(|r| r.as_ref()).ok_or(CliError {
        msg: "No region configured for client".to_string(),
    })?;
    let lambda_client = aws_sdk_lambda::Client::new(config);
    let metrics_client = aws_sdk_cloudwatch::Client::new(config);
    Ok(())
}
