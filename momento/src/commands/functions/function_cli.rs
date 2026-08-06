use momento::{
    functions::{
        CurrentFunctionVersion, FunctionMetricsConfigChange, ListFunctionVersionsRequest,
        ListFunctionsRequest, ListWasmsRequest, PutFunctionConfigRequest, PutFunctionRequest,
        PutWasmRequest, WasmSource,
    },
    FunctionClient,
};

use crate::{
    commands::functions::utils::{
        build_invocation_headers, build_invocation_path, call_function_api, format_metrics_config,
        read_wasm_file, InvocationOptions,
    },
    error::CliError,
    utils::console::console_data,
};

use http::Method;
use log::info;
use std::str::FromStr; // to use http::Method::from_str

pub async fn put_function(
    client: FunctionClient,
    cache_name: String,
    name: String,
    wasm_source: WasmSource,
    description: Option<String>,
    environment_variables: Vec<(String, String)>,
    metrics_change: Option<FunctionMetricsConfigChange>,
) -> Result<(), CliError> {
    let mut request = PutFunctionRequest::new(&cache_name, &name, wasm_source);
    if let Some(description) = description {
        request = request.description(description);
    }
    request = request.environment(environment_variables);
    if let Some(metrics_change) = metrics_change {
        request = request.metrics_config(metrics_change);
    }
    let response = client.send(request).await.map_err(Into::<CliError>::into)?;
    let uploaded_version = response.latest_version();
    let current_version = response.version();
    let metrics = format_metrics_config(response.metrics_config());
    if uploaded_version == current_version {
        console_data!(
            "Function uploaded or updated! Name: {}, ID: {}, Version: {}, Metrics: {}",
            response.name(),
            response.function_id(),
            uploaded_version,
            metrics,
        );
    } else {
        console_data!(
        "Function version uploaded but not in use. Name: {}, ID: {}, Latest Version: {}, Current Version: {}, Metrics: {}",
        response.name(),
        response.function_id(),
        uploaded_version,
        current_version,
        metrics,
    );
    }
    Ok(())
}

pub async fn put_function_config(
    client: FunctionClient,
    cache_name: String,
    function_name: Option<String>,
    function_id: Option<String>,
    new_version: Option<CurrentFunctionVersion>,
    metrics_change: Option<FunctionMetricsConfigChange>,
) -> Result<(), CliError> {
    if new_version.is_none() && metrics_change.is_none() {
        return Err(CliError::new(
            "Specify a version (--pin-version or --use-latest-version) and/or a metrics configuration change (--metrics-iam-role, --disable-metrics, or --remove-metrics-config) to update",
        ));
    }

    let mut request = if let Some(name) = function_name {
        PutFunctionConfigRequest::from_function_name(&cache_name, &name)
    } else if let Some(id) = function_id {
        PutFunctionConfigRequest::from_function_id(&cache_name, &id)
    } else {
        return Err(CliError::new("Function name or ID must be specified"));
    };

    if let Some(new_version) = new_version {
        request = request.current_version(new_version);
    }
    if let Some(metrics_change) = metrics_change {
        request = request.metrics_config(metrics_change);
    }

    let response = client.send(request).await.map_err(Into::<CliError>::into)?;
    console_data!(
        "Function config updated! Name: {}, ID: {}, Latest Version: {}, Current Version: {}, Metrics: {}",
        response.name(),
        response.function_id(),
        response.latest_version(),
        response.version(),
        format_metrics_config(response.metrics_config()),
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
    info!("with request method: {method}");

    let full_path = build_invocation_path(cache_name, name, options.path)?;
    let response_text = call_function_api(
        Method::from_str(&method)?,
        endpoint,
        auth_token,
        full_path,
        headers,
        data,
    )
    .await?;
    console_data!("{response_text}");
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
                "\nName: {}, ID: {}, Latest Version: {}, Current Version: {}, Description: \"{}\", Last Uploaded: {}, Metrics: {}",
                function.name(),
                function.function_id(),
                function.latest_version(),
                function.version(),
                function.description(),
                function.last_updated_at(),
                format_metrics_config(function.metrics_config()),
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
                "\nFunction Version: {}, Description: \"{}\", Wasm ID: {}, Wasm Version: {}, Environment Variables: {:#?}",
                version.version_id().version(),
                version.description(),
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
