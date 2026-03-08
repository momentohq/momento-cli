use std::{future::Future, time::Duration};

use momento::{
    cache::configurations, topics, CacheClient, CredentialProvider, FunctionClient, MomentoError,
    TopicClient,
};

use crate::{config::Credentials, error::CliError};

pub async fn get_cache_client(
    credentials: Credentials,
    endpoint: Option<String>,
) -> Result<CacheClient, CliError> {
    let mut credential_provider = credentials.authenticate()?;
    if let Some(momento_override) = endpoint {
        credential_provider = credential_provider.base_endpoint(&momento_override);
    }

    CacheClient::builder()
        .default_ttl(Duration::from_secs(120))
        .configuration(configurations::Laptop::latest())
        .credential_provider(credential_provider)
        .build()
        .map_err(Into::<CliError>::into)
}

pub async fn get_function_client(
    credential_provider: CredentialProvider,
) -> Result<FunctionClient, CliError> {
    FunctionClient::builder()
        .credential_provider(credential_provider)
        .build()
        .map_err(Into::<CliError>::into)
}

pub async fn get_topic_client(
    credential_provider: CredentialProvider,
) -> Result<TopicClient, CliError> {
    TopicClient::builder()
        .configuration(topics::configurations::Laptop::latest())
        .credential_provider(credential_provider)
        .build()
        .map_err(Into::<CliError>::into)
}

pub async fn interact_with_momento<U, FutureT>(
    debug_note: &str,
    momento_interaction: FutureT,
) -> Result<U, CliError>
where
    FutureT: Future<Output = Result<U, MomentoError>>,
{
    log::debug!("{}", debug_note);

    let result = momento_interaction.await;
    result.map_err(Into::<CliError>::into)
}
