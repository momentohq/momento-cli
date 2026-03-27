use std::collections::HashMap;
use std::fs;
use std::str::FromStr; // to use HeaderName::from_str

use momento::functions::WasmSource;

use crate::error::CliError;

use http::method::InvalidMethod;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, InvalidHeaderName, InvalidHeaderValue};

/// put-function
pub fn read_wasm_file(wasm_file: String) -> Result<Vec<u8>, CliError> {
    let binary = fs::read(wasm_file).map_err(Into::<CliError>::into)?;
    if binary.is_empty() {
        return Err(CliError {
    msg: "Must provide a .wasm file compiled with wasm32-wasip2 to upload using the --wasm-file flag".to_string(),
  });
    }
    Ok(binary)
}

pub fn determine_wasm_source(
    wasm_file: Option<String>,
    id_uploaded_wasm: Option<String>,
    version_uploaded_wasm: Option<u32>,
) -> Result<WasmSource, CliError> {
    match (wasm_file, id_uploaded_wasm, version_uploaded_wasm) {
    (Some(wasm_file), None, None) => Ok(WasmSource::Inline(read_wasm_file(wasm_file)?)),
    (None, Some(id_uploaded_wasm), Some(version_uploaded_wasm)) => Ok(WasmSource::Reference {
      wasm_id: id_uploaded_wasm,
      version: version_uploaded_wasm,
    }),
    _ => Err(CliError {
      msg: "Must provide a .wasm file compiled with wasm32-wasip2 to upload using the --wasm-file flag or a previously uploaded Wasm using the --id-uploaded-wasm and --version-uploaded-wasm flags".to_string(),
    }),
    }
}

/// invoke-function
pub struct InvocationOptions {
    pub data: Option<String>,
    pub headers: Option<String>,
    pub path: Option<String>,
}

pub fn build_invocation_headers(headers_str: &str) -> Result<HeaderMap, CliError> {
    if headers_str.is_empty() {
        return Ok(HeaderMap::new());
    }
    let headers_map = match serde_json::from_str::<HashMap<String, String>>(headers_str) {
        Ok(map) => map,
        Err(e) => {
            return Err(CliError {
                msg: format!("Header {:?}: {e}", e.classify()),
            })
        }
    };
    let mut headers = HeaderMap::with_capacity(headers_map.len());
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
        headers.append(
            HeaderName::from_str(&lower_key)?,
            HeaderValue::from_str(value)?,
        );
    }
    Ok(headers)
}

pub fn build_invocation_url(
    endpoint: String,
    cache_name: String,
    name: String,
    path: Option<String>,
) -> String {
    let function_url = format!("{endpoint}/functions/{cache_name}/{name}");
    match path {
        None => function_url,
        Some(path) => format!("{function_url}/{}", path.trim_start_matches("/")),
    }
}

impl From<reqwest::Error> for CliError {
    fn from(e: reqwest::Error) -> Self {
        CliError { msg: e.to_string() }
    }
}

impl From<InvalidMethod> for CliError {
    fn from(e: InvalidMethod) -> Self {
        CliError {
            msg: format!("Invalid HTTP method: {e}"),
        }
    }
}

impl From<InvalidHeaderName> for CliError {
    fn from(e: InvalidHeaderName) -> Self {
        CliError {
            msg: format!("Header Name: {e}"),
        }
    }
}

impl From<InvalidHeaderValue> for CliError {
    fn from(e: InvalidHeaderValue) -> Self {
        CliError {
            msg: format!("Header Value: {e}"),
        }
    }
}
