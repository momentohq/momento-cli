use super::utils::{
    call_key_create_api, call_key_list_api, call_key_revoke_api, ApiKey, Expiry,
    ListApiKeysResponse, RevokeApiKeyResponse,
};
use crate::commands::utils::MomentoHttpResponse::{Parsed, Unparseable};
use crate::utils::file::prompt_user_for_input;
use crate::{error::CliError, utils::console::console_data};

pub async fn create_key(
    endpoint: String,
    auth_token: String,
    description: String,
    role_id: String,
    expiry: Expiry,
) -> Result<(), CliError> {
    let data = ApiKey {
        role_id,
        description,
        expiry,
        exclude_refresh_token: true, // TODO
    };
    let response = call_key_create_api(endpoint, auth_token, data).await?;
    match response {
        Parsed(key) => {
            console_data!("API Key:\n\n{key}");
        }
        Unparseable(response_text) => {
            console_data!("Creating API key!");
            if !response_text.is_empty() {
                console_data!("\n\n{response_text}");
            }
        }
    };
    Ok(())
}

pub async fn revoke_key(endpoint: String, auth_token: String, id: String) -> Result<(), CliError> {
    let response = call_key_revoke_api(endpoint, auth_token, id.clone()).await?;
    match response {
        Parsed(RevokeApiKeyResponse {}) => console_data!("Revoked API key {id}!"),
        Unparseable(response_text) => {
            console_data!("Attempting to revoke API key {id}:\n\n{response_text}");
        }
    };
    Ok(())
}

pub async fn list_keys(
    endpoint: String,
    auth_token: String,
    limit_per_page: Option<u32>,
) -> Result<(), CliError> {
    let mut next_token = None;
    let mut page_index = 1;
    let mut page_text = "".to_string();
    loop {
        match call_key_list_api(
            endpoint.clone(),
            auth_token.clone(),
            limit_per_page,
            next_token.clone(),
        )
        .await?
        {
            Parsed(ListApiKeysResponse {
                key_info: keys,
                next_token: token,
            }) => {
                if keys.is_empty() {
                    console_data!("No API keys found");
                    break;
                } else {
                    console_data!("API keys{page_text}:");
                    for key in keys.iter() {
                        console_data!("\n{key}");
                    }
                    next_token = token;
                    if next_token.is_none() {
                        break;
                    }
                    let next = prompt_user_for_input("\nView more?", "y", false).await?;
                    if next.to_lowercase() != "y" && next.to_lowercase() != "yes" {
                        break;
                    }
                    page_index += 1;
                    page_text = format!(", page {page_index}");
                    console_data!();
                }
            }
            Unparseable(response_text) => {
                console_data!("Listing your API keys:\n\n{response_text}");
                break;
            }
        };
    }
    Ok(())
}
