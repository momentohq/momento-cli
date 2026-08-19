use crate::commands::utils::{
    call_momento_http_api, call_momento_http_api_raw, MomentoHttpData, MomentoHttpResponse,
};

use crate::error::CliError;
use http::Method;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct DatabaseResponse {
    pub name: String,
    pub pool_name: String,
}

#[derive(Debug, Deserialize)]
pub struct ListDatabasesResponse {
    pub databases: Vec<DatabaseResponse>,
}

fn build_request_url(endpoint: &str, database_name: Option<String>) -> String {
    match database_name {
        None => format!("{endpoint}/database"),
        Some(name) => format!("{endpoint}/database/{name}"),
    }
}

pub async fn call_database_api(
    method: Method,
    endpoint: &str,
    auth_token: String,
    database_name: String,
    data: Option<serde_json::Value>,
) -> Result<MomentoHttpResponse<DatabaseResponse>, CliError> {
    call_momento_http_api(
        method,
        build_request_url(endpoint, Some(database_name)),
        auth_token,
        None,
        data.map(MomentoHttpData::Json),
    )
    .await
}

pub async fn call_database_delete_api(
    endpoint: &str,
    auth_token: String,
    database_name: String,
) -> Result<String, CliError> {
    call_momento_http_api_raw(
        Method::DELETE,
        build_request_url(endpoint, Some(database_name)),
        auth_token,
        None,
        None,
    )
    .await
}

pub async fn call_database_list_api(
    endpoint: &str,
    auth_token: String,
) -> Result<MomentoHttpResponse<ListDatabasesResponse>, CliError> {
    call_momento_http_api(
        Method::GET,
        build_request_url(endpoint, None),
        auth_token,
        None,
        None,
    )
    .await
}
