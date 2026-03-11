use momento::CredentialProvider;
use serde::{Deserialize, Serialize};

use crate::error::CliError;

pub const ENV_VAR_NAME_MOMENTO_CONFIG_DIR: &str = "MOMENTO_CONFIG_DIR";
pub const DEFAULT_CACHE_NAME: &str = "default-cache";

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct Config {
    pub cache: String,
    pub ttl: u64,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub enum Credentials {
    // Accepts api key v2 and endpoint
    ApiKeyV2(String, String),
    // Can accept v1 keys as well
    DisposableToken(String),
}

impl Credentials {
    pub fn override_and_authenticate(
        &self,
        api_key_override: Option<String>,
        endpoint_override: Option<String>,
    ) -> Result<CredentialProvider, CliError> {
        match (api_key_override, endpoint_override) {
            (Some(new_api_key), Some(new_endpoint)) => {
                match CredentialProvider::from_disposable_token(new_api_key.clone()) {
                    Ok(credential_provider) => {
                        // Case: --api-key is disposable token, --endpoint provided
                        Ok(credential_provider.base_endpoint(&new_endpoint))
                    }
                    Err(_) => {
                        // Case: --api-key is v2 API key, --endpoint provided
                        CredentialProvider::from_api_key_v2(new_api_key, new_endpoint)
                            .map_err(Into::<CliError>::into)
                    }
                }
            }
            (Some(new_api_key), None) => {
                match CredentialProvider::from_disposable_token(new_api_key.clone()) {
                    Ok(credential_provider) => {
                        // Case: --api-key is disposable token, defaulting to --profile's endpoint
                        Ok(credential_provider)
                    }
                    Err(_) => match self {
                        Credentials::ApiKeyV2(_, original_endpoint) => {
                            // Case: --api-key is v2 API key, defaulting to --profile's endpoint
                            CredentialProvider::from_api_key_v2(new_api_key, original_endpoint)
                                .map_err(Into::<CliError>::into)
                        }
                        _ => Err(CliError {
                            // Case: --api-key is v2 API key, no endpoint found
                            msg: "To test a v2 API key, provide an endpoint or start with a v2 profile".to_string(),
                        })
                    }
                }
            }
            (None, Some(new_endpoint)) => match self {
                // Case: defaulting to --profile's API key, overriding endpoint
                Credentials::ApiKeyV2(original_api_key, _) => {
                    CredentialProvider::from_api_key_v2(original_api_key, new_endpoint)
                        .map_err(Into::<CliError>::into)
                }
                Credentials::DisposableToken(original_api_key) => {
                    match CredentialProvider::from_disposable_token(original_api_key) {
                        Ok(credential_provider) => {
                            Ok(credential_provider.base_endpoint(&new_endpoint))
                        }
                        Err(err) => Err(CliError { msg: err.message }),
                    }
                }
            },
            (None, None) => match self {
                // Case: defaulting to --profile's API key and endpoint
                Credentials::ApiKeyV2(original_api_key, original_endpoint) => {
                    CredentialProvider::from_api_key_v2(original_api_key, original_endpoint)
                        .map_err(Into::<CliError>::into)
                }
                Credentials::DisposableToken(original_api_key) => {
                    CredentialProvider::from_disposable_token(original_api_key)
                        .map_err(Into::<CliError>::into)
                }
            },
        }
    }
}
