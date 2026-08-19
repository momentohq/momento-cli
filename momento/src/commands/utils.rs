use crate::error::CliError;
use crate::utils::console::console_data;

use http::Method;
use log::{info, warn};
use reqwest;
use serde::{de::DeserializeOwned, Deserialize};
use std::fmt::Debug;

pub fn print_valkey_cli_sample(valkey_endpoint: &str, database_name: &str) {
    console_data!(
        "VALKEYCLI_AUTH=$MOMENTO_API_KEY \\\n  \
           valkey-cli --tls \\\n  \
           -h {valkey_endpoint} \\\n  \
           --user {database_name}"
    );
}

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

pub async fn call_momento_http_api<T: DeserializeOwned + Debug>(
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
            info!("Response sent back from {method} {request_url}:\n{response:#?}");
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
