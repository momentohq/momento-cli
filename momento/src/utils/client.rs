use std::{future::Future, time::Duration};

use momento::{CredentialProviderBuilder, response::MomentoError, SimpleCacheClient, SimpleCacheClientBuilder};

use crate::{error::CliError, utils::console::console_data};

pub async fn get_momento_client(
    auth_token: String,
    endpoint: Option<String>,
) -> Result<SimpleCacheClient, CliError> {
    let mut credential_provider_builder = CredentialProviderBuilder::from_string(auth_token);
    if let Some(momento_override) = endpoint {
        credential_provider_builder = credential_provider_builder.with_momento_endpoint(momento_override);
    }
    let credential_provider = credential_provider_builder.build()?;
    SimpleCacheClientBuilder::new_with_explicit_agent_name(
        credential_provider,
        Duration::from_secs(120),
        "cli",
    )
    .map_or_else(
        |error| Err(Into::<CliError>::into(error)),
        |builder| Ok(builder.build()),
    )
}

pub fn print_whatever_this_is_as_json<T>(value: &T)
where
    T: serde::Serialize,
{
    console_data!(
        "{}",
        serde_json::to_string_pretty(value).expect("Could not print whatever this is as json")
    );
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
