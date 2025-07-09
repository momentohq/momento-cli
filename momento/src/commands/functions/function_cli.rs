use momento::{
    functions::{
        ListFunctionVersionsRequest, ListFunctionsRequest, ListWasmsRequest, PutFunctionRequest,
        PutWasmRequest, WasmSource,
    },
    FunctionClient,
};

use crate::{
    commands::functions::utils::{parse_environment_variables, read_wasm_file},
    error::CliError,
    utils::console::console_data,
};

pub async fn put_function(
    client: FunctionClient,
    cache_name: String,
    name: String,
    wasm_source: WasmSource,
    description: Option<String>,
    environment_variables: Option<Vec<String>>,
) -> Result<(), CliError> {
    let mut request = PutFunctionRequest::new(&cache_name, &name, wasm_source);
    if let Some(description) = description {
        request = request.description(description);
    }
    if let Some(environment_variables) = environment_variables {
        let env_vars = parse_environment_variables(environment_variables)?;
        request = request.environment(env_vars);
    }
    client.send(request).await.map_err(Into::<CliError>::into)?;
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
                version.version_id().id(),
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
    client.send(request).await.map_err(Into::<CliError>::into)?;
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
