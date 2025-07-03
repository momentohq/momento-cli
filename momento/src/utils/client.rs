use std::{future::Future, time::Duration};

use momento::{cache::configurations, CacheClient, CredentialProvider, MomentoError};

use crate::error::CliError;

pub async fn get_momento_client(
    auth_token: String,
    endpoint: Option<String>,
) -> Result<CacheClient, CliError> {
    let mut credential_provider = CredentialProvider::from_string(auth_token)?;
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
