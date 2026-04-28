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
        let disposable_hint = "If you vended a disposable token, make sure it's base64 encoded with an endpoint and api_key";

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
                                    format!("Could not parse --api-key as a disposable token or v2 API key. {disposable_hint}."),
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
                            if original_endpoint.is_empty() {
                                return Err(CliError::new(
                                    "--profile's endpoint is empty. Consider rerunning 'momento configure'."
                                ));
                            };
                            CredentialProvider::from_api_key_v2(new_api_key, original_endpoint)
                                .map_err(|v2_err| {
                                    CliError::new(
                                        format!("Could not parse --api-key as a disposable token or v2 API key. {disposable_hint}."),
                                    )
                                    .with_details(format!("SDK error while parsing as disposable token: {disposable_err:#?}\nSDK error while parsing as v2 API key: {v2_err:#?}"))
                                })
                        }
                        _ => Err(CliError::new(
                            // Case: --api-key is not a valid disposable token, no endpoint found
                            format!("If you're testing a v2 API key, provide an endpoint or start with a v2 profile. {disposable_hint}."),
                        )
                        .with_details(format!("SDK error while parsing as disposable token: {disposable_err:#?}"))),
                    },
                }
            }
            (None, Some(new_endpoint)) => match self {
                // Case: defaulting to --profile's API key, overriding endpoint
                Credentials::ApiKeyV2(original_api_key, _) => {
                    if original_api_key.is_empty() {
                        return Err(CliError::new(
                            "--profile's v2 API key is empty. Consider rerunning 'momento configure'."
                        ));
                    };
                    CredentialProvider::from_api_key_v2(original_api_key, new_endpoint).map_err(
                        |v2_err| {
                            CliError::new("Could not parse --profile's v2 API key. Consider regenerating it and rerunning 'momento configure'.").with_details(
                                format!("SDK error while parsing as v2 API key: {v2_err:#?}"),
                            )
                        },
                    )
                }
                Credentials::DisposableToken(original_api_key) => {
                    if original_api_key.is_empty() {
                        return Err(CliError::new(
                            "--profile's token is empty. Consider rerunning 'momento configure'.",
                        ));
                    };
                    match CredentialProvider::from_disposable_token(original_api_key) {
                        Ok(credential_provider) => {
                            Ok(credential_provider.base_endpoint(&new_endpoint))
                        }
                        Err(disposable_err) => Err(
                            CliError::new(
                                format!("Could not parse --profile's token. {disposable_hint}, and consider rerunning 'momento configure'.")
                            ).with_details(format!("SDK error while parsing as disposable token: {disposable_err:#?}"))
                        ),
                    }
                }
            },
            (None, None) => match self {
                // Case: defaulting to --profile's API key and endpoint
                Credentials::ApiKeyV2(original_api_key, original_endpoint) => {
                    if original_api_key.is_empty() {
                        return Err(CliError::new(
                            "--profile's v2 API key is empty. Consider rerunning 'momento configure'."
                        ));
                    };
                    if original_endpoint.is_empty() {
                        return Err(CliError::new(
                            "--profile's endpoint is empty. Consider rerunning 'momento configure'."
                        ));
                    };
                    CredentialProvider::from_api_key_v2(original_api_key, original_endpoint)
                        .map_err(|v2_err| {
                            CliError::new(
                                "Could not parse --profile's v2 API key. Consider regenerating it and rerunning 'momento configure'."
                            ).with_details(format!("SDK error while parsing as v2 API key: {v2_err:#?}"))
                        })
                }
                Credentials::DisposableToken(original_api_key) => {
                    if original_api_key.is_empty() {
                        return Err(CliError::new(
                            "--profile's token is empty. Consider rerunning 'momento configure'.",
                        ));
                    };
                    CredentialProvider::from_disposable_token(original_api_key).map_err(
                        |disposable_err| {
                            CliError::new(
                                "Could not parse --profile's token. If you vended a disposable token, make sure it's base64 encoded with an endpoint and api_key, and consider rerunning 'momento configure'."
                            ).with_details(format!("SDK error while parsing as disposable token: {disposable_err:#?}"))
                        },
                    )
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_V1_API_KEY: &str = "eyJlbmRwb2ludCI6Im1vbWVudG9fZW5kcG9pbnQiLCJhcGlfa2V5IjoiZXlKaGJHY2lPaUpJVXpJMU5pSjkuZXlKemRXSWlPaUowWlhOMElITjFZbXBsWTNRaUxDSjJaWElpT2pFc0luQWlPaUlpZlEuaGcyd01iV2Utd2VzUVZ0QTd3dUpjUlVMalJwaFhMUXdRVFZZZlFMM0w3YyJ9Cg==";
    const TEST_V2_API_KEY: &str = "eyJhbGciOiJIUzUxMiIsInR5cCI6IkpXVCJ9.eyJ0IjoiZyIsImp0aSI6InNvbWUtaWQifQ.GMr9nA6HE0ttB6llXct_2Sg5-fOKGFbJCdACZFgNbN1fhT6OPg_hVc8ThGzBrWC_RlsBpLA1nzqK3SOJDXYxAw";
    const TEST_ENDPOINT: &str = "test_endpoint";

    // Overriding both API key and endpoint:

    #[test]
    fn test_happy_path_v2_profile_with_v1_override_and_endpoint_override() {}

    #[test]
    fn test_happy_path_v1_profile_with_v1_override_and_endpoint_override() {}

    #[test]
    fn test_happy_path_v2_profile_with_v2_override_and_endpoint_override() {}

    #[test]
    fn test_happy_path_v1_profile_with_v2_override_and_endpoint_override() {}

    #[test]
    fn test_v2_profile_with_invalid_override_and_endpoint_override() {}

    #[test]
    fn test_v1_profile_with_invalid_override_and_endpoint_override() {}

    // Overriding only API key:

    #[test]
    fn test_happy_path_v2_profile_with_v1_override() {
        let creds = Credentials::ApiKeyV2(TEST_V2_API_KEY.to_string(), TEST_ENDPOINT.to_string());
        let credential_provider = creds
            .override_and_authenticate(Some(TEST_V1_API_KEY.to_string()), None)
            .expect("should accept valid v1 API key");

        assert_eq!(
            "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ0ZXN0IHN1YmplY3QiLCJ2ZXIiOjEsInAiOiIifQ.hg2wMbWe-wesQVtA7wuJcRULjRphXLQwQTVYfQL3L7c",
            credential_provider.auth_token()
        );
        assert_eq!(
            "https://api.cache.momento_endpoint",
            credential_provider.cache_http_endpoint()
        );
    }

    #[test]
    fn test_happy_path_v1_profile_with_v1_override() {
        let creds = Credentials::DisposableToken("wfheofhriugheifweif".to_string());
        let credential_provider = creds
            .override_and_authenticate(Some(TEST_V1_API_KEY.to_string()), None)
            .expect("should accept valid v1 API key");

        assert_eq!(
            "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ0ZXN0IHN1YmplY3QiLCJ2ZXIiOjEsInAiOiIifQ.hg2wMbWe-wesQVtA7wuJcRULjRphXLQwQTVYfQL3L7c",
            credential_provider.auth_token()
        );
        assert_eq!(
            "https://api.cache.momento_endpoint",
            credential_provider.cache_http_endpoint()
        );
    }

    #[test]
    fn test_happy_path_v2_profile_with_v2_override() {
        let creds =
            Credentials::ApiKeyV2("wfheofhriugheifweif".to_string(), TEST_ENDPOINT.to_string());
        let credential_provider = creds
            .override_and_authenticate(Some(TEST_V2_API_KEY.to_string()), None)
            .expect("should accept valid v2 API key");

        assert_eq!(TEST_V2_API_KEY, credential_provider.auth_token());
        assert_eq!(
            "https://api.cache.test_endpoint",
            credential_provider.cache_http_endpoint()
        );
    }

    #[test]
    fn test_v2_profile_with_v2_override_and_empty_profile_endpoint() {
        let creds = Credentials::ApiKeyV2("wfheofhriugheifweif".to_string(), "".to_string());
        let result = creds.override_and_authenticate(Some(TEST_V2_API_KEY.to_string()), None);

        let CliError {
            msg: error_message, ..
        } = result.expect_err("should fail");
        assert_eq!(
            "--profile's endpoint is empty. Consider rerunning 'momento configure'.",
            error_message
        );
    }

    #[test]
    fn test_v2_profile_with_invalid_override() {
        let creds = Credentials::ApiKeyV2(TEST_V2_API_KEY.to_string(), TEST_ENDPOINT.to_string());
        let result = creds.override_and_authenticate(Some("wfheofhriugheifweif".to_string()), None);

        let CliError {
            msg: error_message, ..
        } = result.expect_err("should fail");
        assert_eq!(
            "Could not parse --api-key as a disposable token or v2 API key. If you vended a disposable token, make sure it's base64 encoded with an endpoint and api_key.",
            error_message
        );
    }

    #[test]
    fn test_v1_profile_with_invalid_override() {}

    #[test]
    fn test_v1_profile_with_v2_override_and_missing_endpoint_override() {
        let creds = Credentials::DisposableToken("wfheofhriugheifweif".to_string());
        let result = creds.override_and_authenticate(Some(TEST_V2_API_KEY.to_string()), None);

        let CliError {
            msg: error_message, ..
        } = result.expect_err("should fail");
        assert_eq!(
            "If you're testing a v2 API key, provide an endpoint or start with a v2 profile. If you vended a disposable token, make sure it's base64 encoded with an endpoint and api_key.",
            error_message
        );
    }

    // Overriding only endpoint:

    #[test]
    fn test_happy_path_v2_profile_with_endpoint_override() {}

    #[test]
    fn test_v2_profile_with_endpoint_override_and_empty_profile_api_key() {
        let creds = Credentials::ApiKeyV2("".to_string(), "old_endpoint".to_string());
        let result = creds.override_and_authenticate(None, Some(TEST_ENDPOINT.to_string()));

        let CliError {
            msg: error_message, ..
        } = result.expect_err("should fail");
        assert_eq!(
            "--profile's v2 API key is empty. Consider rerunning 'momento configure'.",
            error_message
        );
    }

    #[test]
    fn test_v2_profile_with_endpoint_override_and_invalid_profile_api_key() {}

    #[test]
    fn test_happy_path_v1_profile_with_endpoint_override() {}

    #[test]
    fn test_v1_profile_with_endpoint_override_and_empty_profile_api_key() {}

    #[test]
    fn test_v1_profile_with_endpoint_override_and_invalid_profile_api_key() {}

    // Using profile as-is with no overrides:

    #[test]
    fn test_happy_path_v2_profile() {
        let creds = Credentials::ApiKeyV2(TEST_V2_API_KEY.to_string(), TEST_ENDPOINT.to_string());
        let credential_provider = creds
            .override_and_authenticate(None, None)
            .expect("should accept valid v2 profile");

        assert_eq!(TEST_V2_API_KEY, credential_provider.auth_token());
        assert_eq!(
            "https://api.cache.test_endpoint",
            credential_provider.cache_http_endpoint()
        );
    }

    #[test]
    fn test_happy_path_v1_profile() {
        let creds = Credentials::DisposableToken(TEST_V1_API_KEY.to_string());
        let credential_provider = creds
            .override_and_authenticate(None, None)
            .expect("should accept valid v1 profile");

        assert_eq!(
            "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ0ZXN0IHN1YmplY3QiLCJ2ZXIiOjEsInAiOiIifQ.hg2wMbWe-wesQVtA7wuJcRULjRphXLQwQTVYfQL3L7c",
            credential_provider.auth_token()
        );
        assert_eq!(
            "https://api.cache.momento_endpoint",
            credential_provider.cache_http_endpoint()
        );
    }

    #[test]
    fn test_v2_profile_with_empty_profile_api_key() {}

    #[test]
    fn test_v2_profile_with_empty_profile_endpoint() {}

    #[test]
    fn test_v2_profile_with_invalid_profile_api_key() {
        let creds =
            Credentials::ApiKeyV2("wfheofhriugheifweif".to_string(), TEST_ENDPOINT.to_string());
        let result = creds.override_and_authenticate(None, None);

        let CliError {
            msg: error_message, ..
        } = result.expect_err("should fail");
        assert_eq!(
            "Could not parse --profile's v2 API key. Consider regenerating it and rerunning 'momento configure'.",
            error_message
        );
    }

    #[test]
    fn test_v1_profile_with_empty_profile_api_key() {}

    #[test]
    fn test_v1_profile_with_invalid_profile_api_key() {}
}
