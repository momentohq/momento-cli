use crate::{
    error::CliError,
    utils::client::{get_momento_client, interact_with_momento, print_whatever_this_is_as_json},
};

pub async fn create_signing_key(
    ttl_minutes: u32,
    auth_token: String,
    endpoint: Option<String>,
) -> Result<(), CliError> {
    let mut client = get_momento_client(auth_token, endpoint).await?;

    print_whatever_this_is_as_json(
        &interact_with_momento(
            "creating signing key...",
            client.create_signing_key(ttl_minutes),
        )
        .await?,
    );

    Ok(())
}

pub async fn revoke_signing_key(
    key_id: String,
    auth_token: String,
    endpoint: Option<String>,
) -> Result<(), CliError> {
    let mut client = get_momento_client(auth_token, endpoint).await?;

    interact_with_momento(
        "revoking signing key...",
        client.revoke_signing_key(&key_id),
    )
    .await
}

pub async fn list_signing_keys(
    auth_token: String,
    endpoint: Option<String>,
) -> Result<(), CliError> {
    let mut client = get_momento_client(auth_token, endpoint).await?;

    print_whatever_this_is_as_json(
        &interact_with_momento("listing signing keys...", client.list_signing_keys(None))
            .await
            .map(|list_result| list_result.signing_keys)?,
    );
    Ok(())
}
