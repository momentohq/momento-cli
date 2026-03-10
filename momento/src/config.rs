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
    pub fn authenticate(
        &self,
        token_override: Option<String>,
    ) -> Result<CredentialProvider, CliError> {
        match token_override {
            Some(token) => match CredentialProvider::from_disposable_token(token.clone()) {
                Ok(credential_provider) => Ok(credential_provider),
                Err(_) => match self {
                    Credentials::ApiKeyV2(_, endpoint) => {
                        CredentialProvider::from_api_key_v2(token, endpoint)
                            .map_err(Into::<CliError>::into)
                    }
                    _ => Err(CliError {
                        msg: "Need an endpoint. To test a v2 API key, start with a v2 profile"
                            .to_string(),
                    }),
                },
            },
            None => match self {
                Credentials::ApiKeyV2(api_key, endpoint) => {
                    CredentialProvider::from_api_key_v2(api_key, endpoint)
                        .map_err(Into::<CliError>::into)
                }
                Credentials::DisposableToken(token) => {
                    CredentialProvider::from_disposable_token(token).map_err(Into::<CliError>::into)
                }
            },
        }
    }
}
