use std::{future::Future, num::NonZeroU64};

use momento::{
    response::error::MomentoError,
    simple_cache_client::{SimpleCacheClient, SimpleCacheClientBuilder},
};

use crate::{error::CliError, utils::console::console_data};

pub async fn get_momento_client(
    auth_token: String,
    endpoint: Option<String>,
) -> Result<SimpleCacheClient, CliError> {
    SimpleCacheClientBuilder::new_with_explicit_agent_name(
        auth_token,
        NonZeroU64::new(100).ok_or_else(|| CliError {
            msg: "".to_string(),
        })?,
        "cli",
        endpoint,
    )
    .map_or_else(
        |error| {
            Err(CliError {
                msg: error.to_string(),
            })
        },
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
    result.map_err(|error| CliError {
        msg: error.to_string(),
    })
}
