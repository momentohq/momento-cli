use crate::commands::utils::{call_momento_http_api, MomentoHttpData, MomentoHttpResponse};

use crate::error::CliError;
use http::Method;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct DatabaseResponse {
    pub name: String,
    pub pool_name: String,
}

pub async fn call_database_api(
    method: Method,
    endpoint: String,
    auth_token: String,
    database_name: String,
    data: Option<serde_json::Value>,
) -> Result<MomentoHttpResponse<DatabaseResponse>, CliError> {
    call_momento_http_api(
        method,
        format!("{endpoint}/database/{database_name}"),
        auth_token,
        None,
        data.map(MomentoHttpData::Json),
    )
    .await
}
