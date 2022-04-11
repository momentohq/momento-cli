use log::debug;
use momento::simple_cache_client::SimpleCacheClient;
use std::num::NonZeroU64;

use crate::error::CliError;

async fn get_momento_instance(auth_token: String) -> Result<SimpleCacheClient, CliError> {
    match SimpleCacheClient::new(auth_token, NonZeroU64::new(100).unwrap()).await {
        Ok(m) => Ok(m),
        Err(e) => Err(CliError { msg: e.to_string() }),
    }
}

pub async fn create_signing_key(ttl_minutes: u32, auth_token: String) -> Result<(), CliError> {
    debug!("creating signing key...");
    let mut momento = get_momento_instance(auth_token).await?;
    match momento.create_signing_key(ttl_minutes).await {
        Ok(res) => {
            println!("{}", serde_json::to_string_pretty(&res).unwrap());
        }
        Err(e) => return Err(CliError { msg: e.to_string() }),
    };
    Ok(())
}

pub async fn revoke_signing_key(key_id: String, auth_token: String) -> Result<(), CliError> {
    debug!("revoking signing key...");
    let mut momento = get_momento_instance(auth_token).await?;
    match momento.revoke_signing_key(&key_id).await {
        Ok(_) => (),
        Err(e) => return Err(CliError { msg: e.to_string() }),
    };
    Ok(())
}
