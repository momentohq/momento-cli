use crate::commands::utils::{
    call_momento_http_api, call_momento_http_api_raw, MomentoHttpData, MomentoHttpResponse,
};
use crate::error::CliError;
use crate::utils::console::console_data;

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

fn build_request_url(endpoint: String, database_name: Option<String>) -> String {
    match database_name {
        None => format!("{endpoint}/database"),
        Some(name) => format!("{endpoint}/database/{name}"),
    }
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
        build_request_url(endpoint, Some(database_name)),
        auth_token,
        None,
        data.map(MomentoHttpData::Json),
    )
    .await
}

pub async fn call_database_delete_api(
    endpoint: String,
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
    endpoint: String,
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

pub fn print_valkey_cli_sample(valkey_hostname: String, database_name: &str) {
    console_data!(
        "\nExport your API key from ~/.momento/credentials, then use your favorite RESP client:\n\
        \n\
         VALKEYCLI_AUTH=$MOMENTO_API_KEY \\\n  \
           valkey-cli --tls \\\n  \
           -h {valkey_hostname} \\\n  \
           --user {database_name}"
    );
}
