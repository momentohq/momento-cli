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
                    Err(disposable_err) => {
                        // Case: --api-key is not a valid disposable token, --endpoint provided
                        CredentialProvider::from_api_key_v2(new_api_key, new_endpoint).map_err(
                            |v2_err| {
                                CliError::new(
                                    "Could not parse --api-key as a disposable token or v2 API key. If you vended a disposable token, make sure it's base64 encoded with an endpoint and api_key.",
                                )
                                .with_details(format!("SDK error while parsing as disposable token: {disposable_err:#?}\nSDK error while parsing as v2 API key: {v2_err:#?}"))
                            },
                        )
                    }
                }
            }
            (Some(new_api_key), None) => {
                match CredentialProvider::from_disposable_token(new_api_key.clone()) {
                    Ok(credential_provider) => {
                        // Case: --api-key is disposable token, defaulting to --profile's endpoint
                        Ok(credential_provider)
                    }
                    Err(disposable_err) => match self {
                        Credentials::ApiKeyV2(_, original_endpoint) => {
                            // Case: --api-key is not a valid disposable token, defaulting to --profile's endpoint
                            CredentialProvider::from_api_key_v2(new_api_key, original_endpoint)
                                .map_err(|v2_err| {
                                    CliError::new(
                                        "Could not parse --api-key as a disposable token or v2 API key. If you vended a disposable token, make sure it's base64 encoded with an endpoint and api_key.",
                                    )
                                    .with_details(format!("SDK error while parsing as disposable token: {disposable_err:#?}\nSDK error while parsing as v2 API key: {v2_err:#?}"))
                                })
                        }
                        _ => Err(CliError::new(
                            // Case: --api-key is not a valid disposable token, no endpoint found
                            "If you're testing a v2 API key, provide an endpoint or start with a v2 profile. If you vended a disposable token, make sure it's base64 encoded with an endpoint and api_key.",
                        )
                        .with_details(format!("SDK error while parsing as disposable token: {disposable_err:#?}"))),
                    },
                }
            }
            (None, Some(new_endpoint)) => match self {
                // Case: defaulting to --profile's API key, overriding endpoint
                Credentials::ApiKeyV2(original_api_key, _) => {
                    CredentialProvider::from_api_key_v2(original_api_key, new_endpoint)
                        .map_err(|v2_err| v2_err.into())
                }
                Credentials::DisposableToken(original_api_key) => {
                    match CredentialProvider::from_disposable_token(original_api_key) {
                        Ok(credential_provider) => {
                            Ok(credential_provider.base_endpoint(&new_endpoint))
                        }
                        Err(disposable_err) => Err(disposable_err.into()),
                    }
                }
            },
            (None, None) => match self {
                // Case: defaulting to --profile's API key and endpoint
                Credentials::ApiKeyV2(original_api_key, original_endpoint) => {
                    CredentialProvider::from_api_key_v2(original_api_key, original_endpoint)
                        .map_err(|v2_err| v2_err.into())
                }
                Credentials::DisposableToken(original_api_key) => {
                    CredentialProvider::from_disposable_token(original_api_key)
                        .map_err(|disposable_err| disposable_err.into())
                }
            },
        }
    }
}
