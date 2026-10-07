use super::utils::DatabaseResponse;

use std::fmt;

impl fmt::Display for DatabaseResponse {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Name: {}", self.name)?;
        write!(f, "\nCapacity Pool: {}", self.pool_name)?;
        write!(
            f,
            "\n{}",
            match self.metrics_config.to_string() {
                text if text.contains("\n") => format!("Metrics Config:\n{}", text),
                text => format!("Metrics Config: {}", text),
            }
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::utils::CustomerMetricsConfig;

    fn snapshot_settings() -> insta::Settings {
        let mut settings = insta::Settings::clone_current();
        settings.set_prepend_module_to_snapshot(false);
        settings
    }

    #[test]
    fn test_display_database_with_all_fields() {
        let response = DatabaseResponse {
            name: "my database".to_string(),
            pool_name: "my pool".to_string(),
            metrics_config: CustomerMetricsConfig::CloudWatch {
                customer_iam_role: Some("arn:aws:iam::123456789012:my_momento_metrics".to_string()),
                region: Some("us-east-1".to_string()),
            },
        };

        snapshot_settings().bind(|| insta::assert_snapshot!(response.to_string()));
    }

    #[test]
    fn test_display_database_with_metrics_without_region() {
        let response = DatabaseResponse {
            name: "my database".to_string(),
            pool_name: "my pool".to_string(),
            metrics_config: CustomerMetricsConfig::CloudWatch {
                customer_iam_role: Some("arn:aws:iam::123456789012:my_momento_metrics".to_string()),
                region: None,
            },
        };

        snapshot_settings().bind(|| insta::assert_snapshot!(response.to_string()));
    }

    #[test]
    fn test_display_database_with_metrics_disabled() {
        let response = DatabaseResponse {
            name: "my database".to_string(),
            pool_name: "my pool".to_string(),
            metrics_config: CustomerMetricsConfig::Disabled,
        };
        snapshot_settings().bind(|| insta::assert_snapshot!(response.to_string()));
    }

    #[test]
    fn test_display_database_with_metrics_inherited() {
        let response = DatabaseResponse {
            name: "my database".to_string(),
            pool_name: "my pool".to_string(),
            metrics_config: CustomerMetricsConfig::Inherit,
        };
        snapshot_settings().bind(|| insta::assert_snapshot!(response.to_string()));
    }
}
