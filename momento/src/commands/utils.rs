use crate::error::CliError;

use http::Method;
use log::{info, warn};
use reqwest;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{fmt, fmt::Debug};

// Structs & Args:

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CustomerMetricsConfig {
    Disabled,
    CloudWatch {
        customer_iam_role: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        region: Option<String>,
    },
    Inherit,
}

fn missing_metrics_iam_role_arg() -> CliError {
    CliError::new(
        "Missing --metrics-iam-role.\n\nTo enable metrics configuration, you must specify:\n\
         --metrics-iam-role (and optionally, --metrics-region)",
    )
}

pub fn determine_metrics_config(
    metrics_iam_role: Option<String>,
    metrics_region: Option<String>,
    disable_metrics: bool,
) -> Result<Option<CustomerMetricsConfig>, CliError> {
    let metrics_config = match (&metrics_iam_role, &metrics_region, disable_metrics) {
        (None, None, false) => None, // Fall back to default
        (Some(_), _, false) => Some(CustomerMetricsConfig::CloudWatch {
            customer_iam_role: metrics_iam_role,
            region: metrics_region,
        }),
        (None, Some(_), false) => return Err(missing_metrics_iam_role_arg()),
        (None, None, true) => Some(CustomerMetricsConfig::Disabled),
        _ => return Err(CliError::new(
            "Conflicting argument(s). To configure your metrics, you must specify exactly one of:\
             \n--disable-metrics\n--metrics-iam-role (and optionally, --metrics-region)",
        )),
    };
    Ok(metrics_config)
}

pub fn determine_metrics_config_update(
    metrics_iam_role: Option<String>,
    metrics_region: Option<String>,
    remove_metrics_region: bool,
    disable_metrics: bool,
    remove_metrics_config: bool,
) -> Result<Option<CustomerMetricsConfig>, CliError> {
    let has_aws_field =
        metrics_iam_role.is_some() || metrics_region.is_some() || remove_metrics_region;
    let metrics_config =
        match (has_aws_field, disable_metrics, remove_metrics_config) {
            (false, false, false) => None, // May be updating other fields (e.g. provisioning) instead
            (true, false, false)
                if matches!(
                    (&metrics_iam_role, &metrics_region, remove_metrics_region),
                    (Some(_), _, false) | (_, Some(_), false) | (_, None, true)
                ) =>
            {
                Some(CustomerMetricsConfig::CloudWatch {
                    customer_iam_role: metrics_iam_role,
                    region: metrics_region,
                })
            }
            (false, true, false) => Some(CustomerMetricsConfig::Disabled),
            (false, false, true) => Some(CustomerMetricsConfig::Inherit),
            _ => return Err(CliError::new(
                "Conflicting argument(s). To update your metrics configuration, you must specify:\
                 \n--disable-metrics OR --remove-metrics-config OR some combination of:\
                 \n  --metrics-iam-role and/or --metrics-region (or --remove-metrics-region)",
            )),
        };
    Ok(metrics_config)
}

pub fn determine_metrics_config_update_with_defaults(
    inputs: Option<CustomerMetricsConfig>,
    defaults: CustomerMetricsConfig,
    remove_metrics_region: bool,
) -> Result<Option<CustomerMetricsConfig>, CliError> {
    match inputs.map(|config| config.with_defaults(defaults, remove_metrics_region)) {
        Some(CustomerMetricsConfig::CloudWatch {
            customer_iam_role: None,
            ..
        }) => Err(missing_metrics_iam_role_arg()),
        metrics_config => Ok(metrics_config),
    }
}

impl CustomerMetricsConfig {
    pub fn with_defaults(
        self,
        defaults: CustomerMetricsConfig,
        remove_metrics_region: bool,
    ) -> CustomerMetricsConfig {
        if let CustomerMetricsConfig::CloudWatch {
            customer_iam_role: new_role,
            region: new_region,
        } = self
        {
            CustomerMetricsConfig::CloudWatch {
                customer_iam_role: new_role.or(defaults.customer_iam_role()),
                region: if remove_metrics_region {
                    None
                } else {
                    new_region.or(defaults.region())
                },
            }
        } else {
            self
        }
    }

    fn customer_iam_role(&self) -> Option<String> {
        if let CustomerMetricsConfig::CloudWatch {
            customer_iam_role, ..
        } = self
        {
            customer_iam_role.clone()
        } else {
            None
        }
    }

    fn region(&self) -> Option<String> {
        if let CustomerMetricsConfig::CloudWatch { region, .. } = self {
            region.clone()
        } else {
            None
        }
    }
}

impl fmt::Display for CustomerMetricsConfig {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                CustomerMetricsConfig::Inherit => "inherits configuration".to_string(),
                CustomerMetricsConfig::Disabled => "disabled".to_string(),
                CustomerMetricsConfig::CloudWatch {
                    customer_iam_role: Some(role),
                    region: None,
                } => format!("enabled (IAM role: {role})"),
                CustomerMetricsConfig::CloudWatch {
                    customer_iam_role: Some(role),
                    region: Some(region),
                } => format!("enabled (IAM role: {role}, region: {region})"),
                CustomerMetricsConfig::CloudWatch {
                    // With our checks leading up to this, this should never happen.
                    customer_iam_role: None,
                    region,
                } => format!(
                    "enabled (IAM role: (unknown), region: {})",
                    region.clone().unwrap_or("(unknown)".to_string())
                ),
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
    fn test_determine_metrics_config_update_with_region() {
        let config = determine_metrics_config_update(
            Some(ARN.to_string()),
            Some("us-east-1".to_string()),
            false,
            false,
            false,
        )
        .expect("should accept valid metrics config args");
        assert_eq!(
            Some(CustomerMetricsConfig::CloudWatch {
                customer_iam_role: Some(ARN.to_string()),
                region: Some("us-east-1".to_string()),
            }),
            config
        );
    }

    #[test]
    fn test_metrics_config_region() {
        assert_eq!(
            Some("us-east-1".to_string()),
            CustomerMetricsConfig::CloudWatch {
                customer_iam_role: Some(ARN.to_string()),
                region: Some("us-east-1".to_string()),
            }
            .region()
        );

        assert!(CustomerMetricsConfig::CloudWatch {
            customer_iam_role: Some(ARN.to_string()),
            region: None,
        }
        .region()
        .is_none());

        assert!(CustomerMetricsConfig::Disabled.region().is_none());
        assert!(CustomerMetricsConfig::Inherit.region().is_none());
    }

    #[test]
    fn test_metrics_config_with_defaults() {
        let with_role_only = |role: &str| CustomerMetricsConfig::CloudWatch {
            customer_iam_role: Some(role.to_string()),
            region: None,
        };
        let with_region_only = |region: &str| CustomerMetricsConfig::CloudWatch {
            customer_iam_role: None,
            region: Some(region.to_string()),
        };
        let with_role_and_region = |role: &str, region: &str| CustomerMetricsConfig::CloudWatch {
            customer_iam_role: Some(role.to_string()),
            region: Some(region.to_string()),
        };

        assert_eq!(
            with_role_and_region("arn:same", "us-same-1"),
            with_role_and_region("arn:same", "us-same-1")
                .with_defaults(with_role_and_region("arn:same", "us-same-1"), false),
            "should keep role and region"
        );
        assert_eq!(
            with_role_and_region("arn:new", "us-old-1"),
            with_role_only("arn:new")
                .with_defaults(with_role_and_region("arn:old", "us-old-1"), false),
            "should override default role and fall back to default region"
        );
        assert_eq!(
            with_role_and_region("arn:old", "us-new-1"),
            with_region_only("us-new-1")
                .with_defaults(with_role_and_region("arn:old", "us-old-1"), false),
            "should override default region and fall back to default role"
        );
        assert_eq!(
            with_role_only("arn:same"),
            with_role_only("arn:same").with_defaults(with_role_only("arn:same"), false),
            "should have no region if none specified and no default"
        );

        assert_eq!(
            with_role_only("arn:same"),
            with_role_and_region("arn:same", "us-new-1")
                .with_defaults(with_role_and_region("arn:same", "us-old-1"), true),
            "should remove region if asked"
        );
        assert_eq!(
            with_role_only("arn:new"),
            with_role_only("arn:new")
                .with_defaults(with_role_and_region("arn:old", "us-old-1"), true),
            "should ignore default region if asked to remove region"
        );
        assert_eq!(
            with_role_only("arn:old"),
            with_region_only("us-new-1")
                .with_defaults(with_role_and_region("arn:old", "us-old-1"), true),
            "should override the default role, even if asked to remove region"
        );

        assert_eq!(
            CustomerMetricsConfig::Disabled,
            CustomerMetricsConfig::Disabled
                .with_defaults(with_role_and_region("arn:old", "us-old-1"), false),
            "should disable metrics, ignoring default role and default region"
        );
        assert_eq!(
            CustomerMetricsConfig::Inherit,
            CustomerMetricsConfig::Inherit
                .with_defaults(with_role_and_region("arn:old", "us-old-1"), false),
            "should inherit metrics, ignoring default role and default region"
        );
    }

    #[test]
    fn test_serialize_metrics_config() {
        let with_region = CustomerMetricsConfig::CloudWatch {
            customer_iam_role: Some(ARN.to_string()),
            region: Some("us-east-1".to_string()),
        };
        assert_eq!(
            serde_json::json!({"cloud_watch": {"customer_iam_role": ARN, "region": "us-east-1"}}),
            serde_json::to_value(&with_region).expect("metrics config should serialize")
        );

        let without_region = CustomerMetricsConfig::CloudWatch {
            customer_iam_role: Some(ARN.to_string()),
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
                customer_iam_role: Some(ARN.to_string()),
                region: None,
            },
            metrics_config
        );
    }

    #[test]
    fn test_display_metrics_config() {
        let with_region = CustomerMetricsConfig::CloudWatch {
            customer_iam_role: Some(ARN.to_string()),
            region: Some("us-east-1".to_string()),
        };
        assert_eq!(
            format!("enabled (IAM role: {ARN}, region: us-east-1)"),
            with_region.to_string()
        );

        let without_region = CustomerMetricsConfig::CloudWatch {
            customer_iam_role: Some(ARN.to_string()),
            region: None,
        };
        assert_eq!(
            format!("enabled (IAM role: {ARN})"),
            without_region.to_string()
        );
    }
}
