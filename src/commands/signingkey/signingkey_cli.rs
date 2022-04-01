use log::info;
use momento::simple_cache_client::SimpleCacheClient;

use crate::error::CliError;

async fn get_momento_instance(auth_token: String) -> Result<SimpleCacheClient, CliError> {
    match SimpleCacheClient::new(auth_token, 100).await {
        Ok(m) => Ok(m),
        Err(e) => Err(CliError { msg: e.to_string() }),
    }
}

pub async fn create_signing_key(ttl_minutes: u32, auth_token: String) -> Result<(), CliError> {
    info!("creating signing key...");
    let mut momento = get_momento_instance(auth_token).await?;
    match momento.create_signing_key(ttl_minutes).await {
        Ok(res) => {
            println!("endpoint = {}", res.endpoint);
            println!("key = {}", res.key);
            println!("expires_at = {}", res.expires_at);
        }
        Err(e) => return Err(CliError { msg: e.to_string() }),
    };
    Ok(())
}

pub async fn revoke_signing_key(key_id: String, auth_token: String) -> Result<(), CliError> {
    info!("revoking signing key...");
    let mut momento = get_momento_instance(auth_token).await?;
    match momento.revoke_signing_key(&key_id).await {
        Ok(_) => (),
        Err(e) => return Err(CliError { msg: e.to_string() }),
    };
    Ok(())
}
