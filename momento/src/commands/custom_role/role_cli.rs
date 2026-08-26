use super::utils::{call_role_list_api, ListCustomRolesResponse};
use crate::commands::utils::MomentoHttpResponse::{Parsed, Unparseable};
use crate::{error::CliError, utils::console::console_data};

pub async fn list_roles(endpoint: String, auth_token: String) -> Result<(), CliError> {
    let response = call_role_list_api(endpoint, auth_token).await?;
    match response {
        Parsed(ListCustomRolesResponse { roles: roles_list }) => {
            if roles_list.is_empty() {
                console_data!("No custom roles found");
            } else {
                console_data!("Custom roles available for your Momento API keys:");
                for role in roles_list.iter() {
                    console_data!("\n{role}");
                }
            }
        }
        Unparseable(response_text) => {
            console_data!(
                "Listing custom roles available for your Momento API keys:\n\n{response_text}"
            );
        }
    };
    Ok(())
}
