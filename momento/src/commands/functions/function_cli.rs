use momento::{
    functions::{
        ListFunctionVersionsRequest, ListFunctionsRequest, ListWasmsRequest, PutFunctionRequest,
        PutWasmRequest, WasmSource,
    },
    FunctionClient,
};

use crate::{
    commands::functions::utils::{
        build_invocation_headers, build_invocation_url, read_wasm_file, InvocationOptions,
    },
    error::CliError,
    utils::console::console_data,
};

use http::Method;
use log::info;
use reqwest;
use serde::Deserialize;
use serde_json;
use std::str::FromStr; // to use http::Method::from_str

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

pub async fn invoke_function(
    endpoint: String,
    auth_token: String,
    cache_name: String,
    name: String,
    method: String,
    options: InvocationOptions,
) -> Result<(), CliError> {
    let headers = build_invocation_headers(options.headers.unwrap_or_default().as_str())?;
    let data = options.data.unwrap_or_default();

    info!("Invoking function. Name: {name}, Cache Namespace: {cache_name}");
    if !data.is_empty() {
        info!("with payload:\n{data}");
    };
    if !headers.is_empty() {
        info!("with headers:\n{headers:#?}");
    }

    let request_url = build_invocation_url(endpoint, cache_name, name, options.path);
    info!("at URL: {request_url}");

    info!("with request method: {method}");

    let req_client = reqwest::Client::new();
    let response = req_client
        .request(Method::from_str(&method)?, &request_url)
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
        console_data!("No Wasm sources found");
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
