use super::utils::{
    call_role_delete_api, call_role_list_api, determine_role_id, DeleteCustomRoleResponse,
    DeleteStatus, ListCustomRolesResponse, RoleSelector,
};
use crate::commands::utils::MomentoHttpResponse::{Parsed, Unparseable};
use crate::{error::CliError, utils::console::console_data};

pub async fn delete_role(
    endpoint: String,
    auth_token: String,
    selector: RoleSelector,
) -> Result<(), CliError> {
    let id = determine_role_id(endpoint.clone(), auth_token.clone(), &selector).await?;
    let response = call_role_delete_api(endpoint, auth_token, id.clone()).await?;
    let selector_text = match selector {
        RoleSelector::ById(id) => format!("with ID {id}"),
        RoleSelector::ByName(name) => format!("{name} (ID {id})"),
    };
    match response {
        Parsed(DeleteCustomRoleResponse {
            status,
            active_references,
        }) => match status {
            DeleteStatus::Deleted => console_data!("Deleted custom role {selector_text}!"),
            DeleteStatus::Blocked => return Err(CliError::new(
                format!("Couldn't delete custom role {selector_text} because it's still in use:\n\n{active_references}")
            )),
        },
        Unparseable(response_text) => {
            console_data!("Attempting to delete custom role {selector_text}:\n\n{response_text}");
        }
    };
    Ok(())
}

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
