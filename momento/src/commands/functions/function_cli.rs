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

use reqwest;
use reqwest::StatusCode;
use std::fmt::Write;

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

pub async fn invoke_function(
    endpoint: String,
    auth_token: String,
    cache_name: String,
    name: String,
    data: Option<String>,
) -> Result<(), CliError> {
    let request_url = format!("{endpoint}/functions/{cache_name}/{name}");
    let req_client = reqwest::Client::new();
    let function_info = format!("Name: {name}, Cache Namespace: {cache_name}");

    // Check function before invoking
    let head_status = req_client
        .head(&request_url)
        .header("authorization", &auth_token)
        .send()
        .await?
        .status();
    if !head_status.is_success() {
        return Err(CliError {
            msg: match head_status {
                StatusCode::UNAUTHORIZED => {
                    "Invalid authentication credentials to connect to cache service".into()
                }
                StatusCode::NOT_FOUND => format!("Function not found. {function_info}"),
                _ => format!("Failed to reach function. {function_info}, Status: {head_status}"),
            },
        });
    }

    // Try to invoke function
    let mut call_info = function_info.clone();
    if data.is_some() {
        let _ = write!(call_info, ", Data: {}", data.unwrap_or("N/A".into()));
    }
    let call_info = call_info; // Make immutable
    console_data!("Invoking function. {call_info}");

    let response = req_client
        .get(&request_url)
        .header("authorization", &auth_token)
        .send()
        .await?;
    let status = response.status();
    if !status.is_success() {
        return Err(CliError {
            msg: format!("Failed with status {status}"),
        });
    }

    console_data!("  Response:\n{}", response.text().await?);
    Ok(())
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
                "Name: {}, ID: {}, Version: {}, Description: {}",
                function.name(),
                function.function_id(),
                function.version(),
                function.description()
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
