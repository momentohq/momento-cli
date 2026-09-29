use crate::commands::utils::{call_momento_http_api, MomentoHttpData, MomentoHttpResponse};
use crate::error::CliError;

use chrono::{prelude::DateTime, NaiveDate};
use http::Method;
use serde::{Deserialize, Serialize};
use serde_json;
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Expiry {
    Never,
    #[serde(untagged)]
    Expires(u64),
}

pub fn determine_expiry(
    expires_at_epoch_seconds: Option<u64>,
    expires_in: Option<Duration>,
    expires_on: Option<NaiveDate>,
    now: u64,
) -> Result<Expiry, CliError> {
    let expiry = match (expires_at_epoch_seconds, expires_in, expires_on) {
        (Some(expiration_epoch_seconds), None, None) => Expiry::Expires(expiration_epoch_seconds),
        (None, Some(expires_in), None) => Expiry::Expires(now.saturating_add(expires_in.as_secs())),
        (None, None, Some(date)) => Expiry::Expires(
            match u64::try_from(
                date.and_hms_opt(0, 0, 0)
                    .expect("midnight is always a valid time")
                    .and_utc()
                    .timestamp(),
            ) {
                Ok(epoch_seconds) => epoch_seconds,
                Err(_) => {
                    return Err(CliError::new(format!("Expiry is in the past: {date}")));
                }
            },
        ),

        (None, None, None) => Expiry::Never,
        _ => {
            return Err(CliError::new(
                // This should never happen; clap allows at most 1 expiry argument.
                "Sorry, something went wrong!",
            ));
        }
    };
    if let Expiry::Expires(epoch_seconds) = expiry {
        if epoch_seconds <= now {
            return Err(CliError::new(format!(
                "Expiry is in the past: {epoch_seconds} epoch seconds{}",
                DateTime::from_timestamp(epoch_seconds as i64, 0)
                    .map(|datetime| format!(", {datetime}"))
                    .unwrap_or("".to_string()),
            )));
        }
    }
    Ok(expiry)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiKey {
    pub role_id: String,
    pub description: String,
    pub expiry: Expiry,
    pub exclude_refresh_token: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ApiKeyInfo {
    #[serde(rename = "key_id")]
    pub id: String,
    pub account_id: String,
    pub description: String,
    pub role_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at_epoch_seconds: Option<u64>,
    pub issued_at_epoch_seconds: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ApiKeyResponse {
    pub api_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    pub key_info: ApiKeyInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListApiKeysResponse {
    pub key_info: Vec<ApiKeyInfo>,
    pub next_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RevokeApiKeyResponse {
    // empty JSON
}

/// API calls
fn build_request_url(endpoint: String) -> String {
    format!("{endpoint}/api-keys")
}

pub async fn call_key_create_api(
    endpoint: String,
    auth_token: String,
    data: ApiKey,
) -> Result<MomentoHttpResponse<ApiKeyResponse>, CliError> {
    let url = build_request_url(endpoint);
    call_momento_http_api(
        Method::POST,
        url,
        auth_token,
        None,
        Some(MomentoHttpData::Json(serde_json::to_value(data)?)),
    )
    .await
}

pub async fn call_key_revoke_api(
    endpoint: String,
    auth_token: String,
    key_id: String,
) -> Result<MomentoHttpResponse<RevokeApiKeyResponse>, CliError> {
    let url = build_request_url(endpoint);
    call_momento_http_api(
        Method::DELETE,
        format!("{url}/{key_id}"),
        auth_token,
        None,
        None,
    )
    .await
}

pub async fn call_key_list_api(
    endpoint: String,
    auth_token: String,
    limit_per_page: Option<u32>,
    next_token: Option<String>,
) -> Result<MomentoHttpResponse<ListApiKeysResponse>, CliError> {
    let url = build_request_url(endpoint);
    let query_string = [
        limit_per_page.map_or("".to_string(), |limit| format!("&limit={limit}")),
        next_token.map_or("".to_string(), |token| format!("&next_token={token}")),
    ]
    .join("");
    call_momento_http_api(
        Method::GET,
        format!("{url}?{query_string}"),
        auth_token,
        None,
        None,
    )
    .await
}
