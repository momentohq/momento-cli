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
    let data = data.unwrap_or_default();
    let call_info = if !data.is_empty() {
        format!("Name: {name}, Cache Namespace: {cache_name}, Payload: {data}")
    } else {
        format!("Name: {name}, Cache Namespace: {cache_name}")
    };
    console_data!("Invoking function. {call_info}");

    let req_client = reqwest::Client::new();
    let response = req_client
        .post(&request_url)
        .body(data)
        .header("authorization", &auth_token)
        .send()
        .await?;
    let status = response.status();
    if status.is_success() {
        console_data!("  Response:\n{}", response.text().await?);
        Ok(())
    } else {
        Err(CliError {
            msg: match status {
                StatusCode::UNAUTHORIZED => "Invalid authentication credentials".into(),
                StatusCode::NOT_FOUND => "Function or cache not found".into(),
                StatusCode::FORBIDDEN => "Insufficient permissions to invoke function".into(),
                StatusCode::BAD_REQUEST => {
                    let error_text = response.text().await?;
                    format!("Invocation failed with 400 Bad Request. {error_text}")
                }
                _ => format!("Invocation failed. {status}"),
            },
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
