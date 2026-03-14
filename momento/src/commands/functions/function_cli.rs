use std::collections::HashMap;

use momento::{
    functions::{
        ListFunctionVersionsRequest, ListFunctionsRequest, ListWasmsRequest, PutFunctionRequest,
        PutWasmRequest, WasmSource,
    },
    FunctionClient,
};

use crate::{
    commands::functions::utils::read_wasm_file, error::CliError, utils::console::console_data,
};

use log::info;
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Client as reqwest_Client,
};
use serde::Deserialize;
use serde_json;

#[derive(Deserialize)]
struct InvokeError {
    detail: Option<String>,
    message: Option<String>,
}

pub async fn put_function(
    client: FunctionClient,
    cache_name: String,
    name: String,
    wasm_source: WasmSource,
    description: Option<String>,
    environment_variables: Vec<(String, String)>,
) -> Result<(), CliError> {
    let mut request = PutFunctionRequest::new(&cache_name, &name, wasm_source);
    if let Some(description) = description {
        request = request.description(description);
    }
    request = request.environment(environment_variables);
    let response = client.send(request).await.map_err(Into::<CliError>::into)?;
    console_data!(
        "Function uploaded or updated! Name: {}, ID: {}, Version: {}",
        response.name(),
        response.function_id(),
        response.version()
    );
    Ok(())
}

fn build_invocation_headers(headers_str: &str) -> Result<HeaderMap, CliError> {
    let mut headers = HeaderMap::new();
    if headers_str.is_empty() {
        return Ok(headers);
    }
    let headers_map = match serde_json::from_str::<HashMap<String, String>>(headers_str) {
        Ok(map) => map,
        Err(e) => {
            return Err(CliError {
                msg: format!("Header {:?}: {e}", e.classify()),
            })
        }
    };
    for (key, value) in headers_map.iter() {
        let lower_key = key.to_lowercase();
        if lower_key == "authorization" {
            return Err(CliError {
                msg: "To use a specific Momento API key, please specify --profile or --api-key, not an authorization header".to_string()
            });
        }
        if headers.contains_key(&lower_key) {
            // HashMap already case-sensitively ignored duplicate keys,
            // so here, we case-insensitively ignore duplicate keys
            continue;
        }
        headers.insert(
            HeaderName::from_bytes(lower_key.as_bytes())?,
            HeaderValue::from_bytes(value.as_bytes())?,
        );
    }
    Ok(headers)
}

pub async fn invoke_function(
    endpoint: String,
    auth_token: String,
    cache_name: String,
    name: String,
    data: Option<String>,
    headers_string: Option<String>,
) -> Result<(), CliError> {
    let headers = build_invocation_headers(headers_string.unwrap_or_default().as_str())?;
    let data = data.unwrap_or_default();

    let function_info = format!("Name: {name}, Cache Namespace: {cache_name}");
    match (!data.is_empty(), !headers.is_empty()) {
        (false, false) => {
            info!("Invoking function. {function_info}");
        }
        (false, true) => {
            info!("Invoking function. {function_info}, Headers: {headers:?}");
        }
        (true, false) => {
            info!("Sending data to function. {function_info}, Payload: {data}");
        }
        (true, true) => {
            info!(
                "Sending data to function. {function_info}, Payload: {data}, Headers: {headers:?}"
            );
        }
    }

    let request_url = format!("{endpoint}/functions/{cache_name}/{name}");
    let req_client = reqwest_Client::new();
    let response = req_client
        .post(&request_url)
        .body(data)
        .header("authorization", &auth_token)
        .headers(headers)
        .send()
        .await?;
    let status = response.status();
    if status.is_success() {
        console_data!("{}", response.text().await?);
        Ok(())
    } else {
        let error_text = response.text().await?;
        let error_message = match serde_json::from_str::<InvokeError>(error_text.as_str()) {
            Ok(error_json) => {
                info!("{error_text}");
                error_json
                    .detail
                    .unwrap_or(error_json.message.unwrap_or(error_text))
            }
            Err(_) => error_text,
        };
        Err(CliError {
            msg: format!("{status}: {error_message}"),
        })
    }
}

pub async fn list_functions(client: FunctionClient, cache_name: String) -> Result<(), CliError> {
    let request = ListFunctionsRequest::new(&cache_name);
    let response = client.send(request).await.map_err(Into::<CliError>::into)?;
    let functions_list = response.into_vec().await.map_err(Into::<CliError>::into)?;

    if functions_list.is_empty() {
        console_data!("No functions found in cache namespace: {cache_name}");
    } else {
        console_data!("Functions in cache namespace: {cache_name}");
        functions_list.iter().for_each(|function| {
            console_data!(
                "Name: {}, ID: {}, Version: {}, Description: {}, Last Updated: {}",
                function.name(),
                function.function_id(),
                function.version(),
                function.description(),
                function.last_updated_at(),
            )
        });
    }
    Ok(())
}

pub async fn list_function_versions(
    client: FunctionClient,
    function_id: String,
) -> Result<(), CliError> {
    let request = ListFunctionVersionsRequest::new(&function_id);
    let response = client.send(request).await.map_err(Into::<CliError>::into)?;
    let function_versions_list = response.into_vec().await.map_err(Into::<CliError>::into)?;

    if function_versions_list.is_empty() {
        console_data!("No versions found for function: {function_id}");
    } else {
        console_data!("Versions for function: {function_id}");
        function_versions_list.iter().for_each(|version| {
            console_data!(
                "Function Version: {}, Wasm ID: {}, Wasm Version: {}, Environment Variables: {:#?}",
                version.version_id().version(),
                version.wasm_version_id().id(),
                version.wasm_version_id().version(),
                version.environment()
            )
        });
    }
    Ok(())
}

pub async fn put_wasm(
    client: FunctionClient,
    name: String,
    wasm_file: String,
    description: Option<String>,
) -> Result<(), CliError> {
    let binary = read_wasm_file(wasm_file)?;
    let mut request = PutWasmRequest::new(&name, binary);
    if let Some(description) = description {
        request = request.description(description);
    }
    let response = client.send(request).await.map_err(Into::<CliError>::into)?;
    console_data!(
        "Wasm uploaded or updated! Name: {}, ID: {}, Version: {}",
        response.name(),
        response.id().id(),
        response.id().version()
    );
    Ok(())
}

pub async fn list_wasms(client: FunctionClient) -> Result<(), CliError> {
    let request = ListWasmsRequest::new();
    let response = client.send(request).await.map_err(Into::<CliError>::into)?;
    let wasms_list = response.into_vec().await.map_err(Into::<CliError>::into)?;

    if wasms_list.is_empty() {
        console_data!("No wasm sources found");
    } else {
        console_data!("Wasm sources:");
        wasms_list.iter().for_each(|wasm| {
            console_data!(
                "Name: {}, ID: {}, Version: {}, Description: {}",
                wasm.name(),
                wasm.id().id(),
                wasm.id().version(),
                wasm.description()
            )
        });
    }
    Ok(())
}
