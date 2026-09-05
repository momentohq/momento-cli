use crate::commands::utils::{call_momento_http_api, MomentoHttpData, MomentoHttpResponse};
use crate::error::CliError;

use http::Method;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "snake_case")]
pub enum Condition {
    IpFilter { allowed_cidr_ranges: Vec<String> },
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "snake_case")]
pub enum PermissionAction {
    Read,
    Write,
    List,
    Invoke,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "snake_case")]
pub enum NameSelector {
    #[serde(rename = "*")]
    All,
    Name(String),
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "snake_case")]
pub enum PrefixSelector {
    #[serde(rename = "*")]
    All,
    Name(String),
    Prefix(String),
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ItemSelector {
    #[serde(rename = "*")]
    All,
    Key(String),
    KeyPrefix(String),
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Rule {
    Cache {
        permissions: Vec<PermissionAction>,
        caches: NameSelector,
        items: ItemSelector,
    },
    Topic {
        permissions: Vec<PermissionAction>,
        topics: PrefixSelector,
        caches: NameSelector,
    },
    Store {
        permissions: Vec<PermissionAction>,
        stores: NameSelector,
        items: ItemSelector,
    },
    Function {
        permissions: Vec<PermissionAction>,
        functions: PrefixSelector,
        caches: NameSelector,
    },
    Database {
        permissions: Vec<PermissionAction>,
        databases: NameSelector,
    },
    AccountManagement {
        permissions: Vec<PermissionAction>,
    },
    AuthManagement {
        permissions: Vec<PermissionAction>,
    },
    ResourceManagement {
        permissions: Vec<PermissionAction>,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Permissions {
    #[serde(default)]
    pub rules: Vec<Rule>,
    #[serde(default)]
    pub conditions: Vec<Condition>,
}

#[derive(Serialize)]
pub struct CustomRole {
    #[serde(rename = "role_name")]
    pub name: String,
    pub description: Option<String>,
    pub permissions: Permissions,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CustomRoleResponse {
    #[serde(rename = "role_name")]
    pub name: String,
    #[serde(rename = "role_id")]
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub permissions: Permissions,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListCustomRolesResponse {
    pub roles: Vec<CustomRoleResponse>,
}

/// delete_role
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DeleteStatus {
    Deleted,
    Blocked,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountMember {
    pub user_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Invitation {
    pub account_member: AccountMember,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiKey {
    pub key_id: String,
    pub account_id: String,
    pub description: String,
    pub issued_at_epoch_seconds: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ActiveReferences {
    #[serde(default)]
    pub account_members: Vec<AccountMember>,
    #[serde(default)]
    pub invitations: Vec<Invitation>,
    #[serde(default)]
    pub api_keys: Vec<ApiKey>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteCustomRoleResponse {
    pub status: DeleteStatus,
    #[serde(flatten)]
    pub active_references: ActiveReferences,
}

/// Role Name vs ID
pub enum RoleSelector {
    ByName(String),
    ById(String),
}

pub fn determine_role_selector(
    id: Option<String>,
    name: Option<String>,
) -> Result<RoleSelector, CliError> {
    match (id, name) {
        (Some(id), None) => Ok(RoleSelector::ById(id)),
        (None, Some(name)) => Ok(RoleSelector::ByName(name)),
        _ => {
            Err(CliError::new(
                // This should never happen; clap requires role id XOR role name.
                "Sorry, something went wrong!",
            ))
        }
    }
}

pub async fn determine_role(
    endpoint: String,
    auth_token: String,
    selector: &RoleSelector,
) -> Result<CustomRoleResponse, CliError> {
    let selector_text = match selector {
        RoleSelector::ById(id) => format!("ID {id}"),
        RoleSelector::ByName(name) => format!("name {name}"),
    };
    let roles_list = match call_role_list_api(endpoint, auth_token).await? {
        MomentoHttpResponse::Parsed(ListCustomRolesResponse { roles: roles_list }) => {
            if roles_list.is_empty() {
                return Err(CliError::new("No custom roles found"));
            } else {
                roles_list
            }
        }
        MomentoHttpResponse::Unparseable(response_text) => {
            return Err(CliError::new(format!(
                "Can't determine which custom role has {selector_text}:\n\n{response_text}"
            )))
        }
    };
    match selector {
        RoleSelector::ById(id) => {
            for role in roles_list.iter() {
                if role.id == id.clone() {
                    return Ok(role.clone());
                }
            }
        }
        RoleSelector::ByName(name) => {
            for role in roles_list.iter() {
                if role.name == name.clone() {
                    return Ok(role.clone());
                }
            }
        }
    }
    Err(CliError::new(format!(
        "No custom role has {selector_text}.\n\nListing custom roles:\n\n{}",
        roles_list
            .iter()
            .map(|role| role.to_string())
            .collect::<Vec<String>>()
            .join("\n\n")
    )))
}

pub fn determine_role_update(
    existing_role: CustomRoleResponse,
    new_name: Option<String>,
    new_description: Option<String>,
    new_permission_set: Option<String>,
) -> Result<CustomRole, CliError> {
    Ok(CustomRole {
        name: new_name.unwrap_or(existing_role.name),
        description: new_description.or(existing_role.description),
        permissions: new_permission_set
            .and_then(|permissions| serde_json::from_str::<Permissions>(permissions.as_str()).ok())
            .unwrap_or(existing_role.permissions),
    })
}

/// API calls
fn build_request_url(endpoint: String) -> String {
    format!("{endpoint}/roles")
}

pub async fn call_role_create_api(
    endpoint: String,
    auth_token: String,
    data: CustomRole,
) -> Result<MomentoHttpResponse<CustomRoleResponse>, CliError> {
    let url = build_request_url(endpoint);
    call_momento_http_api(
        Method::POST,
        url,
        auth_token,
        None,
        Some(MomentoHttpData::Json(serde_json::to_value(data)?)),
    )
    .await
}

pub async fn call_role_update_api(
    endpoint: String,
    auth_token: String,
    role_id: String,
    data: CustomRole,
) -> Result<MomentoHttpResponse<CustomRoleResponse>, CliError> {
    let url = build_request_url(endpoint);
    call_momento_http_api(
        Method::PUT,
        format!("{url}/{role_id}"),
        auth_token,
        None,
        Some(MomentoHttpData::Json(serde_json::to_value(data)?)),
    )
    .await
}

pub async fn call_role_delete_api(
    endpoint: String,
    auth_token: String,
    role_id: String,
) -> Result<MomentoHttpResponse<DeleteCustomRoleResponse>, CliError> {
    let url = build_request_url(endpoint);
    call_momento_http_api(
        Method::DELETE,
        format!("{url}/{role_id}"),
        auth_token,
        None,
        None,
    )
    .await
}

pub async fn call_role_list_api(
    endpoint: String,
    auth_token: String,
) -> Result<MomentoHttpResponse<ListCustomRolesResponse>, CliError> {
    let url = build_request_url(endpoint);
    call_momento_http_api(
        Method::GET,
        format!("{url}?type=custom"),
        auth_token,
        None,
        None,
    )
    .await
}

#[cfg(test)]
pub mod test_utils {
    use super::*;

    pub fn parse_role(json: &str) -> CustomRoleResponse {
        serde_json::from_str(json).expect("should parse role")
    }
}

#[cfg(test)]
mod tests {
    use super::test_utils::*;
    use super::*;

    #[test]
    fn test_deserialize_role_with_all_permissions() {
        let role = parse_role(
            r#"{
                "role_id": "r-everything",
                "role_name": "Everything",
                "description": "I have a description",
                "permissions": {
                    "rules": [
                        {
                            "type": "account_management",
                            "permissions": [
                                "read",
                                "write",
                                "list"
                            ]
                        },
                        {
                            "type": "auth_management",
                            "permissions": [
                                "read",
                                "write",
                                "list"
                            ],
                            "items": "*"
                        },
                        {
                            "type": "resource_management",
                            "permissions": [
                                "read",
                                "write",
                                "list"
                            ],
                            "resources": "*"
                        },
                        {
                            "type": "cache",
                            "permissions": [
                                "read",
                                "write",
                                "list"
                            ],
                            "caches": "*",
                            "items": "*"
                        },
                        {
                            "type": "topic",
                            "permissions": [
                                "read",
                                "write",
                                "list"
                            ],
                            "caches": "*",
                            "topics": "*"
                        },
                        {
                            "type": "store",
                            "permissions": [
                                "read",
                                "write",
                                "list"
                            ],
                            "stores": "*",
                            "items": "*"
                        },
                        {
                            "type": "function",
                            "permissions": [
                                "invoke"
                            ],
                            "caches": "*",
                            "functions": "*"
                        },
                        {
                            "type": "database",
                            "permissions": [
                                "read",
                                "write"
                            ],
                            "databases": "*"
                        }
                    ],
                    "conditions": [
                        {
                            "ip_filter": {
                                "allowed_cidr_ranges": [
                                    "0.0.0.0/0"
                                ]
                            }
                        }
                    ]
                },
                "role_type": "custom"
            }"#,
        );

        assert_eq!("Everything", role.name);
        assert_eq!("r-everything", role.id);
        assert_eq!(
            "I have a description",
            role.description.expect("should have description")
        );
        assert_eq!(8, role.permissions.rules.len());
        assert_eq!(1, role.permissions.conditions.len());

        assert_eq!(
            Rule::AccountManagement {
                permissions: vec![
                    PermissionAction::Read,
                    PermissionAction::Write,
                    PermissionAction::List
                ],
            },
            role.permissions.rules[0]
        );
        assert_eq!(
            Rule::AuthManagement {
                permissions: vec![
                    PermissionAction::Read,
                    PermissionAction::Write,
                    PermissionAction::List
                ],
            },
            role.permissions.rules[1]
        );
        assert_eq!(
            Rule::ResourceManagement {
                permissions: vec![
                    PermissionAction::Read,
                    PermissionAction::Write,
                    PermissionAction::List
                ],
            },
            role.permissions.rules[2]
        );

        assert_eq!(
            Rule::Cache {
                permissions: vec![
                    PermissionAction::Read,
                    PermissionAction::Write,
                    PermissionAction::List
                ],
                caches: NameSelector::All,
                items: ItemSelector::All,
            },
            role.permissions.rules[3]
        );
        assert_eq!(
            Rule::Topic {
                permissions: vec![
                    PermissionAction::Read,
                    PermissionAction::Write,
                    PermissionAction::List
                ],
                caches: NameSelector::All,
                topics: PrefixSelector::All,
            },
            role.permissions.rules[4]
        );
        assert_eq!(
            Rule::Store {
                permissions: vec![
                    PermissionAction::Read,
                    PermissionAction::Write,
                    PermissionAction::List
                ],
                stores: NameSelector::All,
                items: ItemSelector::All,
            },
            role.permissions.rules[5]
        );
        assert_eq!(
            Rule::Function {
                permissions: vec![PermissionAction::Invoke],
                caches: NameSelector::All,
                functions: PrefixSelector::All,
            },
            role.permissions.rules[6]
        );
        assert_eq!(
            Rule::Database {
                permissions: vec![PermissionAction::Read, PermissionAction::Write,],
                databases: NameSelector::All,
            },
            role.permissions.rules[7]
        );

        assert_eq!(
            Condition::IpFilter {
                allowed_cidr_ranges: vec!["0.0.0.0/0".to_string()],
            },
            role.permissions.conditions[0]
        );
    }

    #[test]
    fn test_deserialize_role_with_limited_permissions() {
        let role = parse_role(
            r#"{
                "role_id": "r-limited",
                "role_name": "Limited",
                "description": "role with limited permissions",
                "permissions": {
                    "rules": [
                        {
                            "type": "resource_management",
                            "permissions": [
                                "read",
                                "list"
                            ],
                            "resources": "*"
                        },
                        {
                            "type": "cache",
                            "permissions": [
                                "list"
                            ],
                            "caches": { "name": "foobar" },
                            "items": "*"
                        },
                        {
                            "type": "cache",
                            "permissions": [
                                "read"
                            ],
                            "caches": { "name": "foobar" },
                            "items": { "key_prefix": "hello" }
                        },
                        {
                            "type": "cache",
                            "permissions": [
                                "write"
                            ],
                            "caches": { "name": "foobar" },
                            "items": { "key": "helloworld" }
                        },
                        {
                            "type": "topic",
                            "permissions": [
                                "read",
                                "list"
                            ],
                            "caches": { "name": "foobar" },
                            "topics": { "prefix": "prod-" }
                        },
                        {
                            "type": "topic",
                            "permissions": [
                                "read",
                                "list",
                                "write"
                            ],
                            "caches": { "name": "foobar" },
                            "topics": { "prefix": "preprod-" }
                        },
                        {
                            "type": "topic",
                            "permissions": [
                                "read",
                                "list",
                                "write"
                            ],
                            "caches": "*",
                            "topics": { "name": "dev" }
                        }
                    ],
                    "conditions": [
                        {
                            "ip_filter": {
                                "allowed_cidr_ranges": [
                                    "10.1.2.3/32",
                                    "5.4.3.2/24"
                                ]
                            }
                        }
                    ]
                },
                "role_type": "custom"
            }"#,
        );

        assert_eq!("Limited", role.name);
        assert_eq!("r-limited", role.id);
        assert_eq!(
            "role with limited permissions",
            role.description.expect("should have description")
        );
        assert_eq!(7, role.permissions.rules.len());
        assert_eq!(1, role.permissions.conditions.len());

        // Rules:
        assert_eq!(
            Rule::ResourceManagement {
                permissions: vec![PermissionAction::Read, PermissionAction::List],
            },
            role.permissions.rules[0]
        );
        assert_eq!(
            Rule::Cache {
                permissions: vec![PermissionAction::List],
                caches: NameSelector::Name("foobar".to_string()),
                items: ItemSelector::All
            },
            role.permissions.rules[1]
        );
        assert_eq!(
            Rule::Cache {
                permissions: vec![PermissionAction::Read],
                caches: NameSelector::Name("foobar".to_string()),
                items: ItemSelector::KeyPrefix("hello".to_string())
            },
            role.permissions.rules[2]
        );
        assert_eq!(
            Rule::Cache {
                permissions: vec![PermissionAction::Write],
                caches: NameSelector::Name("foobar".to_string()),
                items: ItemSelector::Key("helloworld".to_string())
            },
            role.permissions.rules[3]
        );

        assert_eq!(
            Rule::Topic {
                permissions: vec![PermissionAction::Read, PermissionAction::List,],
                caches: NameSelector::Name("foobar".to_string()),
                topics: PrefixSelector::Prefix("prod-".to_string())
            },
            role.permissions.rules[4]
        );
        assert_eq!(
            Rule::Topic {
                permissions: vec![
                    PermissionAction::Read,
                    PermissionAction::List,
                    PermissionAction::Write,
                ],
                caches: NameSelector::Name("foobar".to_string()),
                topics: PrefixSelector::Prefix("preprod-".to_string())
            },
            role.permissions.rules[5]
        );
        assert_eq!(
            Rule::Topic {
                permissions: vec![
                    PermissionAction::Read,
                    PermissionAction::List,
                    PermissionAction::Write,
                ],
                caches: NameSelector::All,
                topics: PrefixSelector::Name("dev".to_string())
            },
            role.permissions.rules[6]
        );

        // Conditions:
        assert_eq!(
            Condition::IpFilter {
                allowed_cidr_ranges: vec!["10.1.2.3/32".to_string(), "5.4.3.2/24".to_string()]
            },
            role.permissions.conditions[0]
        );
    }

    #[test]
    fn test_deserialize_role_with_no_description() {
        let role = parse_role(
            r#"{
                "role_id": "r-limited",
                "role_name": "Limited",
                "permissions": {
                    "rules": [
                        {
                            "type": "resource_management",
                            "permissions": [
                                "read",
                                "list"
                            ],
                            "resources": "*"
                        },
                        {
                            "type": "cache",
                            "permissions": [
                                "list"
                            ],
                            "caches": { "name": "foobar" },
                            "items": "*"
                        },
                        {
                            "type": "cache",
                            "permissions": [
                                "read"
                            ],
                            "caches": { "name": "foobar" },
                            "items": { "key_prefix": "hello" }
                        },
                        {
                            "type": "cache",
                            "permissions": [
                                "write"
                            ],
                            "caches": { "name": "foobar" },
                            "items": { "key": "helloworld" }
                        },
                        {
                            "type": "topic",
                            "permissions": [
                                "read",
                                "list"
                            ],
                            "caches": { "name": "foobar" },
                            "topics": { "prefix": "prod-" }
                        },
                        {
                            "type": "topic",
                            "permissions": [
                                "read",
                                "list",
                                "write"
                            ],
                            "caches": { "name": "foobar" },
                            "topics": { "prefix": "preprod-" }
                        },
                        {
                            "type": "topic",
                            "permissions": [
                                "read",
                                "list",
                                "write"
                            ],
                            "caches": "*",
                            "topics": { "name": "dev" }
                        }
                    ],
                    "conditions": [
                        {
                            "ip_filter": {
                                "allowed_cidr_ranges": [
                                    "10.1.2.3/32",
                                    "5.4.3.2/24"
                                ]
                            }
                        }
                    ]
                },
                "role_type": "custom"
            }"#,
        );

        assert_eq!("Limited", role.name);
        assert_eq!("r-limited", role.id);
        assert_eq!(None, role.description);
        assert_eq!(7, role.permissions.rules.len());
        assert_eq!(1, role.permissions.conditions.len());
    }

    #[test]
    fn test_deserialize_role_with_empty_description() {
        let role = parse_role(
            r#"{
                "role_id": "r-limited",
                "role_name": "Limited",
                "description": "",
                "permissions": {
                    "rules": [
                        {
                            "type": "resource_management",
                            "permissions": [
                                "read",
                                "list"
                            ],
                            "resources": "*"
                        },
                        {
                            "type": "cache",
                            "permissions": [
                                "list"
                            ],
                            "caches": { "name": "foobar" },
                            "items": "*"
                        },
                        {
                            "type": "cache",
                            "permissions": [
                                "read"
                            ],
                            "caches": { "name": "foobar" },
                            "items": { "key_prefix": "hello" }
                        },
                        {
                            "type": "cache",
                            "permissions": [
                                "write"
                            ],
                            "caches": { "name": "foobar" },
                            "items": { "key": "helloworld" }
                        },
                        {
                            "type": "topic",
                            "permissions": [
                                "read",
                                "list"
                            ],
                            "caches": { "name": "foobar" },
                            "topics": { "prefix": "prod-" }
                        },
                        {
                            "type": "topic",
                            "permissions": [
                                "read",
                                "list",
                                "write"
                            ],
                            "caches": { "name": "foobar" },
                            "topics": { "prefix": "preprod-" }
                        },
                        {
                            "type": "topic",
                            "permissions": [
                                "read",
                                "list",
                                "write"
                            ],
                            "caches": "*",
                            "topics": { "name": "dev" }
                        }
                    ],
                    "conditions": [
                        {
                            "ip_filter": {
                                "allowed_cidr_ranges": [
                                    "10.1.2.3/32",
                                    "5.4.3.2/24"
                                ]
                            }
                        }
                    ]
                },
                "role_type": "custom"
            }"#,
        );

        assert_eq!("Limited", role.name);
        assert_eq!("r-limited", role.id);
        assert_eq!("", role.description.expect("should have description"));
        assert_eq!(7, role.permissions.rules.len());
        assert_eq!(1, role.permissions.conditions.len());
    }

    #[test]
    fn test_deserialize_role_with_no_conditions() {
        let role = parse_role(
            r#"{
                "role_id": "r-limited",
                "role_name": "Limited",
                "description": "role with limited permissions",
                "permissions": {
                    "rules": [
                        {
                            "type": "cache",
                            "permissions": [
                                "list"
                            ],
                            "caches": { "name": "foobar" },
                            "items": "*"
                        }
                    ]
                },
                "role_type": "custom"
            }"#,
        );

        assert_eq!("Limited", role.name);
        assert_eq!("r-limited", role.id);
        assert_eq!(
            "role with limited permissions",
            role.description.expect("should have description")
        );
        assert_eq!(
            vec![Rule::Cache {
                permissions: vec![PermissionAction::List],
                caches: NameSelector::Name("foobar".to_string()),
                items: ItemSelector::All
            }],
            role.permissions.rules
        );

        assert!(role.permissions.conditions.is_empty());
    }

    #[test]
    fn test_deserialize_role_with_no_rules() {
        let role = parse_role(
            r#"{
                "role_id": "r-limited",
                "role_name": "Limited",
                "permissions": {
                    "conditions": [
                        {
                            "ip_filter": {
                                "allowed_cidr_ranges": [
                                    "10.1.2.3/32",
                                    "5.4.3.2/24"
                                ]
                            }
                        }
                    ]
                },
                "role_type": "custom"
            }"#,
        );

        assert_eq!("Limited", role.name);
        assert_eq!("r-limited", role.id);
        assert_eq!(None, role.description);
        assert_eq!(
            Condition::IpFilter {
                allowed_cidr_ranges: vec!["10.1.2.3/32".to_string(), "5.4.3.2/24".to_string()]
            },
            role.permissions.conditions[0]
        );

        assert!(role.permissions.rules.is_empty());
    }
}
