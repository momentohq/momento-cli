use chrono::{Duration, TimeZone, Utc};
use configparser::ini::Ini;
use regex::Regex;

use crate::{
    config::{Config, Credentials},
    error::CliError,
    utils::file::{get_config_file_path, get_credentials_file_path, read_ini_file},
};

fn get_session_token(credentials: &Ini) -> Option<String> {
    let session_token = credentials.get(".momento_session", "token");
    if session_token.is_some() {
        let expiry = credentials
            .get(".momento_session", "valid_until")
            .map(|s| s.parse::<i64>().unwrap_or(0))
            .map(|expiry_timestamp| Utc.timestamp_opt(expiry_timestamp, 0).single());
        if let Some(Some(expiry_timestamp)) = expiry {
            if Utc::now() + Duration::seconds(10) < expiry_timestamp {
                let expiring = expiry_timestamp - Utc::now();
                log::debug!("Found user session expiring in {}m", expiring.num_minutes());
                return session_token;
            }
            log::debug!("Token already expired at: {}", expiry_timestamp);
        } else {
            log::debug!(
                ".momento_session profile is missing the expiry time. Skipping this session..."
            );
        }
    }
    log::debug!("No session found in .momento_session profile...");
    None
}

pub async fn get_creds_and_config(profile: &str) -> Result<(Credentials, Config), CliError> {
    let creds = get_creds_for_profile(profile).await?;
    let config = get_config_for_profile(profile).await?;

    Ok((creds, config))
}

pub async fn get_creds_for_profile(profile: &str) -> Result<Credentials, CliError> {
    let credentials_file = read_credentials().await?;

    // if session is configured, use that
    if let Some(session_token) = get_session_token(&credentials_file) {
        return Ok(Credentials::DisposableToken(session_token));
    }

    // if only token is set, use disposable token method
    if let Some(token) = credentials_file.get(profile, "token") {
        return Ok(Credentials::DisposableToken(token));
    }

    // if both api_key_v2 and endpoint are set, use api key v2 method
    if let (Some(api_key_v2), Some(endpoint)) = (
        credentials_file.get(profile, "api_key_v2"),
        credentials_file.get(profile, "endpoint"),
    ) {
        return Ok(Credentials::ApiKeyV2(api_key_v2, endpoint));
    }

    // else invalid credentials, prompt to reconfigure
    Err(CliError::new(
        format!("failed to get credentials for profile {profile}, please run 'momento configure' to configure your profile")
    ))
}

async fn read_credentials() -> Result<Ini, CliError> {
    let path = get_credentials_file_path()?;
    read_ini_file(&path).await
}

pub async fn get_config_for_profile(profile: &str) -> Result<Config, CliError> {
    let path = get_config_file_path()?;
    let configs = match read_ini_file(&path).await {
        Ok(c) => c,
        Err(e) => return Err(CliError::new(
            format!("failed to read credentials, please run 'momento configure' to setup credentials. Root cause: {e:?}")
        )),
    };

    let cache_result = match configs.get(profile, "cache") {
        Some(c) => c,
        None => return Err(CliError::new(
            format!("failed to get cache config for profile {profile}, please run 'momento configure' to configure your profile")
        )),
    };

    let ttl_result = match configs.get(profile, "ttl") {
        Some(c) => c,
        None => return Err(CliError::new(
            format!("failed to get ttl config for profile {profile}, please run 'momento configure' to configure your profile")
        )),
    };

    Ok(Config {
        cache: cache_result,
        ttl: ttl_result
            .parse::<u64>()
            .map_err(|e| CliError::new(format!("could not parse a u64: {e:?}")))?,
    })
}

fn determine_cell_prefix_for_region(region: &str) -> String {
    match region {
        "us-east-1" | "ap-northeast-1" => "cell",
        "us-west-2" => "cell-4",
        _ => "cell-1",
    }
    .to_string()
}

/// Formats any sample from https://docs.momentohq.com/platform/regions
pub fn determine_endpoint(endpoint_arg: String) -> String {
    if endpoint_arg.is_empty() {
        return endpoint_arg;
    }
    let prefixes = ["https://", "api.", "cache."];
    let mut endpoint = endpoint_arg.clone();
    if endpoint_arg.contains(".") {
        for p in prefixes {
            endpoint = endpoint.strip_prefix(p).unwrap_or(&endpoint).to_string();
        }
    } else {
        if !endpoint_arg.starts_with("cell-") {
            let prefix = determine_cell_prefix_for_region(&endpoint_arg);
            endpoint = format!("{prefix}-{endpoint}");
        }
        if let Ok(suffix) = Regex::new(r"-[0-9]-1$") {
            if !suffix.is_match(&endpoint_arg) {
                endpoint += "-1";
            }
        }
        endpoint += ".prod.a.momentohq.com";
    }
    endpoint
}

pub fn determine_mga_endpoint(api_endpoint: String) -> String {
    format!(
        "https://mga.registry.{}.a.momentohq.com",
        if api_endpoint.ends_with(".preprod.a.momentohq.com") {
            "preprod"
        } else {
            "prod"
        }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_determine_endpoint_with_valid_endpoint() {
        let endpoint_arg = "cell.preprod.a.momentohq.com";
        let endpoint = determine_endpoint(endpoint_arg.to_string());
        assert_eq!(endpoint_arg, endpoint);

        let endpoint_arg = "cell-us-east-1-1.prod.a.momentohq.com";
        let endpoint = determine_endpoint(endpoint_arg.to_string());
        assert_eq!(endpoint_arg, endpoint);

        let endpoint_arg = "cell-4-us-west-2-1.prod.a.momentohq.com";
        let endpoint = determine_endpoint(endpoint_arg.to_string());
        assert_eq!(endpoint_arg, endpoint);

        let endpoint_arg = "cell-1-ap-southeast-2-1.prod.a.momentohq.com";
        let endpoint = determine_endpoint(endpoint_arg.to_string());
        assert_eq!(endpoint_arg, endpoint);

        let endpoint_arg = "cell-10-us-northeast-3-1.prod.a.momentohq.com";
        let endpoint = determine_endpoint(endpoint_arg.to_string());
        assert_eq!(endpoint_arg, endpoint);

        let endpoint_arg = "cell-foo-bar-us-northeast-3-1.prod.a.momentohq.com";
        let endpoint = determine_endpoint(endpoint_arg.to_string());
        assert_eq!(endpoint_arg, endpoint);
    }

    #[test]
    fn test_determine_endpoint_with_cell_name() {
        let endpoint = determine_endpoint("cell-us-east-1-1".to_string());
        assert_eq!("cell-us-east-1-1.prod.a.momentohq.com", endpoint);

        let endpoint = determine_endpoint("cell-4-us-west-2-1".to_string());
        assert_eq!("cell-4-us-west-2-1.prod.a.momentohq.com", endpoint);

        let endpoint = determine_endpoint("cell-1-ap-southeast-2-1".to_string());
        assert_eq!("cell-1-ap-southeast-2-1.prod.a.momentohq.com", endpoint);
    }

    #[test]
    fn test_determine_endpoint_with_cell_name_no_suffix() {
        let endpoint = determine_endpoint("cell-us-east-1".to_string());
        assert_eq!("cell-us-east-1-1.prod.a.momentohq.com", endpoint);

        let endpoint = determine_endpoint("cell-4-us-west-2".to_string());
        assert_eq!("cell-4-us-west-2-1.prod.a.momentohq.com", endpoint);

        let endpoint = determine_endpoint("cell-1-ap-southeast-2".to_string());
        assert_eq!("cell-1-ap-southeast-2-1.prod.a.momentohq.com", endpoint);
    }

    #[test]
    fn test_determine_endpoint_with_url() {
        let endpoint = determine_endpoint(
            "https://api.cache.cell-us-east-1-1.prod.a.momentohq.com".to_string(),
        );
        assert_eq!("cell-us-east-1-1.prod.a.momentohq.com", endpoint);

        let endpoint = determine_endpoint(
            "https://api.cache.cell-4-us-west-2-1.prod.a.momentohq.com".to_string(),
        );
        assert_eq!("cell-4-us-west-2-1.prod.a.momentohq.com", endpoint);

        let endpoint = determine_endpoint(
            "https://api.cache.cell-1-ap-southeast-2-1.prod.a.momentohq.com".to_string(),
        );
        assert_eq!("cell-1-ap-southeast-2-1.prod.a.momentohq.com", endpoint);
    }

    #[test]
    fn test_determine_endpoint_with_region_only() {
        let endpoint = determine_endpoint("us-east-1".to_string());
        assert_eq!("cell-us-east-1-1.prod.a.momentohq.com", endpoint);

        let endpoint = determine_endpoint("us-west-2".to_string());
        assert_eq!("cell-4-us-west-2-1.prod.a.momentohq.com", endpoint);

        let endpoint = determine_endpoint("ap-southeast-2".to_string());
        assert_eq!("cell-1-ap-southeast-2-1.prod.a.momentohq.com", endpoint);
    }

    #[test]
    fn test_determine_mga_endpoint() {
        let api_endpoints = [
            "cell-us-east-1-1.prod.a.momentohq.com",
            "cell-4-us-west-2-1.prod.a.momentohq.com",
            "cell-foo-bar-us-northeast-3-1.prod.a.momentohq.com",
            "supercalifragilistixexpialidocious.momentohq.com",
        ];
        let expected = "https://mga.registry.prod.a.momentohq.com";
        for endpoint in api_endpoints {
            let actual = determine_mga_endpoint(endpoint.to_string());
            assert_eq!(expected, actual);
        }
    }
}
