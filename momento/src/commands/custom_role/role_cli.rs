use super::utils::{
    call_role_create_api, call_role_delete_api, call_role_list_api, call_role_update_api,
    determine_role, determine_role_update, CustomRole, DeleteCustomRoleResponse, DeleteStatus,
    ListCustomRolesResponse, Permissions, RoleSelector,
};
use crate::commands::utils::MomentoHttpResponse::{Parsed, Unparseable};
use crate::{error::CliError, utils::console::console_data};

use serde_json;

pub async fn create_role(
    endpoint: String,
    auth_token: String,
    name: String,
    description: Option<String>,
    permission_set: String,
) -> Result<(), CliError> {
    let data = CustomRole {
        name,
        description,
        permissions: serde_json::from_str::<Permissions>(&permission_set)?,
    };
    let response = call_role_create_api(endpoint, auth_token, data).await?;
    match response {
        Parsed(role) => {
            console_data!("Creating custom role!\n\n{role}");
        }
        Unparseable(response_text) => {
            console_data!("Creating custom role!");
            if !response_text.is_empty() {
                console_data!("\n\n{response_text}");
            }
        }
    };
    Ok(())
}

pub async fn update_role(
    endpoint: String,
    auth_token: String,
    selector: RoleSelector,
    new_name: Option<String>,
    description: Option<String>,
    permission_set: Option<String>,
) -> Result<(), CliError> {
    let from_name = match &selector {
        RoleSelector::ById(_) => "".to_string(),
        RoleSelector::ByName(name) => format!(" from {name}"),
    };
    let update_text = match (&new_name, &description, &permission_set) {
        (Some(name), None, None) => format!("Renaming custom role{from_name} to {name}!"),
        (Some(name), _, _) => format!("Updating custom role and renaming{from_name} to {name}!"),
        (None, None, None) => {
            // This should never happen; clap requires at least 1 field.
            return Err(CliError::new("Please provide at least 1 field to update."));
        }
        (None, _, _) => "Updating custom role!".to_string(),
    };
    let existing_role = determine_role(endpoint.clone(), auth_token.clone(), &selector).await?;
    let data = determine_role_update(existing_role.clone(), new_name, description, permission_set)?;
    let response = call_role_update_api(endpoint, auth_token, existing_role.id, data).await?;
    match response {
        Parsed(role) => {
            console_data!("{update_text}\n\n{role}");
        }
        Unparseable(response_text) => {
            console_data!("{update_text}");
            if !response_text.is_empty() {
                console_data!("\n\n{response_text}");
            }
        }
    };
    Ok(())
}

pub async fn delete_role(
    endpoint: String,
    auth_token: String,
    selector: RoleSelector,
) -> Result<(), CliError> {
    let id = determine_role(endpoint.clone(), auth_token.clone(), &selector)
        .await?
        .id;
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
