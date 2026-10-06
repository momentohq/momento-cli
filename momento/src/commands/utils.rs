use crate::error::CliError;

use http::Method;
use log::{info, warn};
use reqwest;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{fmt, fmt::Debug};

// Structs & Args:

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CustomerMetricsConfig {
    Disabled,
    CloudWatch {
        customer_iam_role: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        region: Option<String>,
    },
    Inherit,
}

pub fn determine_metrics_config(
    metrics_iam_role: Option<String>,
    metrics_region: Option<String>,
    disable_metrics: bool,
    remove_metrics_config: bool,
) -> Result<Option<CustomerMetricsConfig>, CliError> {
    let metrics_config = match (metrics_iam_role, remove_metrics_config, disable_metrics) {
        (None, true, false) => Some(CustomerMetricsConfig::Inherit),
        (None, false, true) => Some(CustomerMetricsConfig::Disabled),
        (Some(customer_iam_role), false, false) => Some(CustomerMetricsConfig::CloudWatch {
            customer_iam_role,
            region: metrics_region,
        }),
        (None, false, false) => None,
        (Some(_), true, _) | (Some(_), _, true) | (_, true, true) => {
            // This should never happen; clap requires at most 1 metrics config field.
            return Err(CliError::new(
                "Please provide at most 1 metrics config field.",
            ));
        }
    };
    Ok(metrics_config)
}

impl fmt::Display for CustomerMetricsConfig {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                CustomerMetricsConfig::Inherit => "none (follows account-wide default)".to_string(),
                CustomerMetricsConfig::Disabled => "disabled".to_string(),
                CustomerMetricsConfig::CloudWatch {
                    customer_iam_role,
                    region: None,
                } => format!("enabled (IAM role: {customer_iam_role})"),
                CustomerMetricsConfig::CloudWatch {
                    customer_iam_role,
                    region: Some(region),
                } => format!("enabled (IAM role: {customer_iam_role}, region: {region})"),
            }
        )
    }
}

// API Calls:
pub enum MomentoHttpData {
    Json(serde_json::Value),
    String(String),
}

#[derive(Deserialize)]
struct MomentoHttpError {
    pub detail: Option<String>,
    pub message: Option<String>,
}

pub enum MomentoHttpResponse<T> {
    Parsed(T),
    Unparseable(String),
}

async fn call_api(
    method: Method,
    request_url: String,
    auth_token: String,
    headers: Option<reqwest::header::HeaderMap>,
    data: Option<MomentoHttpData>,
) -> Result<String, CliError> {
    let req_client = reqwest::Client::builder().build()?;
    let request_builder = req_client
        .request(method.clone(), &request_url)
        .header("authorization", &auth_token);
    let request_builder = match data {
        None => request_builder,
        Some(MomentoHttpData::Json(data)) => request_builder
            .body(data.to_string())
            .header("content-type", "application/json"),
        Some(MomentoHttpData::String(data)) => request_builder.body(data),
    };
    let request_builder = match headers {
        None => request_builder,
        Some(mut headers) => {
            if headers.remove("authorization").is_some() {
                warn!("Removed authorization header; must be specified via --profile or --api-key");
            }
            request_builder.headers(headers)
        }
    };

    let response = request_builder.send().await?;
    let status = response.status();

    info!(
        "Headers sent back from {method} {request_url}:\n{:#?}",
        response.headers()
    );

    if status.is_success() {
        Ok(response.text().await?)
    } else {
        let error_text = response.text().await?;
        Err(CliError::new(if error_text.is_empty() {
            format!("{status}")
        } else {
            let error_message = match serde_json::from_str::<MomentoHttpError>(error_text.as_str())
            {
                Ok(error) => error
                    .detail
                    .unwrap_or(error.message.unwrap_or(error_text.clone())),
                Err(_) => error_text.clone(),
            };
            format!("{status}: {error_message}")
        })
        .with_details(error_text))
    }
}

pub async fn call_momento_http_api_raw(
    method: Method,
    request_url: String,
    auth_token: String,
    headers: Option<reqwest::header::HeaderMap>,
    data: Option<MomentoHttpData>,
) -> Result<String, CliError> {
    let response_text = call_api(
        method.clone(),
        request_url.clone(),
        auth_token,
        headers,
        data,
    )
    .await?;
    info!("Response sent back from {method} {request_url}:\n{response_text}");
    Ok(response_text)
}

pub async fn call_momento_http_api<T: DeserializeOwned + Debug + Serialize>(
    method: Method,
    request_url: String,
    auth_token: String,
    headers: Option<reqwest::header::HeaderMap>,
    data: Option<MomentoHttpData>,
) -> Result<MomentoHttpResponse<T>, CliError> {
    let response_text = call_api(
        method.clone(),
        request_url.clone(),
        auth_token,
        headers,
        data,
    )
    .await?;
    match serde_json::from_str::<T>(response_text.as_str()) {
        Ok(response) => {
            info!(
                "Response sent back from {method} {request_url}:\n{}",
                serde_json::to_string_pretty::<T>(&response).unwrap_or(response_text)
            );
            Ok(MomentoHttpResponse::Parsed(response))
        }
        Err(err) => {
            warn!(
                "Can't parse response from {method} {request_url}: \
                \n{response_text}\n{err}"
            );
            Ok(MomentoHttpResponse::Unparseable(response_text))
        }
    }
}

impl From<reqwest::Error> for CliError {
    fn from(e: reqwest::Error) -> Self {
        CliError::new(format!("{e} (reqwest error)")).with_details(format!("{e:#?}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ARN: &str = "arn:aws:iam::123456789012:role/my_momento_metrics";

    #[test]
    fn test_determine_metrics_config_with_region() {
        let config = determine_metrics_config(
            Some(ARN.to_string()),
            Some("us-east-1".to_string()),
            false,
            false,
        )
        .expect("should accept valid metrics config args");
        assert_eq!(
            Some(CustomerMetricsConfig::CloudWatch {
                customer_iam_role: ARN.to_string(),
                region: Some("us-east-1".to_string()),
            }),
            config
        );
    }

    #[test]
    fn test_serialize_metrics_config() {
        let with_region = CustomerMetricsConfig::CloudWatch {
            customer_iam_role: ARN.to_string(),
            region: Some("us-east-1".to_string()),
        };
        assert_eq!(
            serde_json::json!({"cloud_watch": {"customer_iam_role": ARN, "region": "us-east-1"}}),
            serde_json::to_value(&with_region).expect("metrics config should serialize")
        );

        let without_region = CustomerMetricsConfig::CloudWatch {
            customer_iam_role: ARN.to_string(),
            region: None,
        };
        assert_eq!(
            serde_json::json!({"cloud_watch": {"customer_iam_role": ARN}}),
            serde_json::to_value(&without_region).expect("metrics config should serialize")
        );
    }

    #[test]
    fn test_deserialize_metrics_config() {
        let metrics_config = serde_json::from_str::<CustomerMetricsConfig>(
            &serde_json::json!({"cloud_watch": {"customer_iam_role": ARN}}).to_string(),
        )
        .expect("should parse metrics config");
        assert_eq!(
            CustomerMetricsConfig::CloudWatch {
                customer_iam_role: ARN.to_string(),
                region: None,
            },
            metrics_config
        );
    }

    #[test]
    fn test_display_metrics_config() {
        let with_region = CustomerMetricsConfig::CloudWatch {
            customer_iam_role: ARN.to_string(),
            region: Some("us-east-1".to_string()),
        };
        assert_eq!(
            format!("enabled (IAM role: {ARN}, region: us-east-1)"),
            with_region.to_string()
        );

        let without_region = CustomerMetricsConfig::CloudWatch {
            customer_iam_role: ARN.to_string(),
            region: None,
        };
        assert_eq!(
            format!("enabled (IAM role: {ARN})"),
            without_region.to_string()
        );
    }
}
