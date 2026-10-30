use crate::commands::utils::{
    call_momento_http_api, call_momento_http_api_raw, CustomerMetricsConfig, MomentoHttpData,
    MomentoHttpResponse,
};
use crate::error::CliError;
use crate::utils::console::console_data;

use http::Method;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct Database {
    pub pool_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics_config: Option<CustomerMetricsConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DatabaseResponse {
    pub name: String,
    #[serde(default)]
    pub database_id: String,
    pub pool_name: String,
    #[serde(default)]
    pub pool_id: String,
    pub metrics_config: CustomerMetricsConfig,
}

#[derive(Debug, Serialize, Deserialize)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_database_with_ids() {
        let response: DatabaseResponse = serde_json::from_str(
            r#"{
                "name": "my database",
                "database_id": "db-5e6f7a8b",
                "pool_name": "my pool",
                "pool_id": "cp-1a2b3c4d",
                "metrics_config": "disabled"
            }"#,
        )
        .expect("database should deserialize");

        assert_eq!(response.database_id, "db-5e6f7a8b");
        assert_eq!(response.pool_id, "cp-1a2b3c4d");
    }

    #[test]
    fn test_deserialize_database_without_ids() {
        let response: DatabaseResponse = serde_json::from_str(
            r#"{
                "name": "my database",
                "pool_name": "my pool",
                "metrics_config": "disabled"
            }"#,
        )
        .expect("database should deserialize");

        assert_eq!(response.database_id, "");
        assert_eq!(response.pool_id, "");
    }
}
