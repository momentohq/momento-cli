use crate::commands::utils::{call_momento_http_api, MomentoHttpData, MomentoHttpResponse};
use crate::error::CliError;

use http::Method;
use serde::{Deserialize, Serialize};
use serde_json;

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
pub enum AllSelector {
    #[serde(rename = "*")]
    All,
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
        items: ItemSelector,
    },
    AccountManagement {
        permissions: Vec<PermissionAction>,
    },
    AuthManagement {
        permissions: Vec<PermissionAction>,
        items: AllSelector,
    },
    ResourceManagement {
        permissions: Vec<PermissionAction>,
        resources: AllSelector,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Permissions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub super_user: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules: Option<Vec<Rule>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conditions: Option<Vec<Condition>>,
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
    pub role_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub permissions: Permissions,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListCustomRolesResponse {
    pub roles: Vec<CustomRoleResponse>,
    pub next_token: Option<String>,
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
    all: bool,
) -> Result<CustomRoleResponse, CliError> {
    let (role_text, list_command) = if all {
        ("role", "momento role list --all")
    } else {
        ("custom role", "momento role list")
    };
    let selector_text = match selector {
        RoleSelector::ById(id) => format!("ID {id}"),
        RoleSelector::ByName(name) => format!("name {name}"),
    };
    let mut full_roles_list = vec![];
    let mut next_token = None;
    loop {
        match call_role_list_api(
            endpoint.clone(),
            auth_token.clone(),
            None,
            all,
            next_token.clone(),
        )
        .await?
        {
            MomentoHttpResponse::Parsed(ListCustomRolesResponse {
                roles: roles_list,
                next_token: token,
            }) => {
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
                full_roles_list = [full_roles_list, roles_list].concat();
                next_token = token;
                if next_token.is_none() {
                    if full_roles_list.is_empty() {
                        return Err(CliError::new(format!("No {role_text}s found")));
                    } else {
                        return Err(CliError::new(format!(
                            "No {role_text} has {selector_text}. Check `{list_command}`"
                        )));
                    }
                }
            }
            MomentoHttpResponse::Unparseable(response_text) => {
                return Err(CliError::new(format!(
                    "Can't determine which custom role has {selector_text}.\n\n{response_text}"
                )));
            }
        }
    }
}

pub fn determine_role_permissions(permission_set: String) -> Result<Permissions, CliError> {
    let help_text = "For more information, try '--help'";
    if permission_set.is_empty() {
        return Err(CliError::new(format!(
            "Invalid --permission-set; must specify rules.\n\n{help_text}"
        )));
    }
    match serde_json::from_str::<Permissions>(&permission_set) {
        Err(error) => Err(CliError::new(format!(
            "Invalid --permission-set: {error}\n\n{help_text}"
        ))),
        Ok(permissions) if permissions.rules.is_none() => Err(CliError::new(format!(
            "Invalid --permission-set; must specify rules.\n\n{help_text}",
        ))),
        Ok(permissions) => Ok(permissions),
    }
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
            .map(determine_role_permissions)
            .transpose()?
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
    limit_per_page: Option<u32>,
    all: bool,
    next_token: Option<String>,
) -> Result<MomentoHttpResponse<ListCustomRolesResponse>, CliError> {
    let url = build_request_url(endpoint);
    let query_string = [
        (if all { "" } else { "type=custom" }).to_string(),
        limit_per_page.map_or("".to_string(), |limit| format!("&limit={limit}")),
        next_token.map_or("".to_string(), |token| format!("&next_token={token}")),
    ]
    .join("");
    call_momento_http_api(
        Method::GET,
        format!("{url}?{query_string}"),
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
        serde_json::from_str(json).expect("should parse custom role")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use momento_cli_opts::ROLE_PERMISSIONS_SAMPLE;

    use super::test_utils::*;
    use std::assert_matches;

    fn parse_permission_rule(json: &str) -> Rule {
        serde_json::from_str(json).expect("should parse permission rule")
    }

    // determine_role_permissions

    #[test]
    fn test_determine_role_permissions_sample() {
        let permissions = determine_role_permissions(ROLE_PERMISSIONS_SAMPLE.to_string())
            .expect("should parse permissions");

        let rules = permissions.rules.expect("should have rules");
        assert_eq!(3, rules.len());

        let conditions = permissions.conditions.expect("should have conditions");
        assert_eq!(1, conditions.len());

        assert_eq!(
            Rule::AccountManagement {
                permissions: vec![PermissionAction::Read, PermissionAction::List],
            },
            rules[0]
        );
        assert_eq!(
            Rule::Cache {
                permissions: vec![PermissionAction::Read, PermissionAction::Write],
                caches: NameSelector::Name("prod-cache".to_string()),
                items: ItemSelector::All,
            },
            rules[1]
        );
        assert_eq!(
            Rule::Function {
                permissions: vec![PermissionAction::Invoke],
                caches: NameSelector::Name("edge-app".to_string()),
                functions: PrefixSelector::Prefix("webhook-".to_string()),
            },
            rules[2]
        );

        assert_eq!(
            Condition::IpFilter {
                allowed_cidr_ranges: vec![
                    "10.0.0.0/8".to_string(),
                    "192.168.1.0/24".to_string(),
                    "2001:db8::/32".to_string()
                ],
            },
            conditions[0]
        );
    }

    #[test]
    fn test_determine_role_permissions_with_all_fields() {
        let permissions = determine_role_permissions(
            r#"{
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
            }"#
            .to_string(),
        )
        .expect("should parse permissions");

        let rules = permissions.rules.expect("should have rules");
        assert_eq!(7, rules.len());

        let conditions = permissions.conditions.expect("should have conditions");
        assert_eq!(1, conditions.len());

        // Rules:
        assert_eq!(
            Rule::ResourceManagement {
                permissions: vec![PermissionAction::Read, PermissionAction::List],
                resources: AllSelector::All,
            },
            rules[0]
        );
        assert_eq!(
            Rule::Cache {
                permissions: vec![PermissionAction::List],
                caches: NameSelector::Name("foobar".to_string()),
                items: ItemSelector::All
            },
            rules[1]
        );
        assert_eq!(
            Rule::Cache {
                permissions: vec![PermissionAction::Read],
                caches: NameSelector::Name("foobar".to_string()),
                items: ItemSelector::KeyPrefix("hello".to_string())
            },
            rules[2]
        );
        assert_eq!(
            Rule::Cache {
                permissions: vec![PermissionAction::Write],
                caches: NameSelector::Name("foobar".to_string()),
                items: ItemSelector::Key("helloworld".to_string())
            },
            rules[3]
        );

        assert_eq!(
            Rule::Topic {
                permissions: vec![PermissionAction::Read, PermissionAction::List,],
                caches: NameSelector::Name("foobar".to_string()),
                topics: PrefixSelector::Prefix("prod-".to_string())
            },
            rules[4]
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
            rules[5]
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
            rules[6]
        );

        // Conditions:
        assert_eq!(
            Condition::IpFilter {
                allowed_cidr_ranges: vec!["10.1.2.3/32".to_string(), "5.4.3.2/24".to_string()]
            },
            conditions[0]
        );
    }

    #[test]
    fn test_determine_role_permissions_with_no_conditions() {
        let permissions = determine_role_permissions(
            r#"{
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
            }"#
            .to_string(),
        )
        .expect("should parse permissions");

        assert_eq!(
            vec![Rule::Cache {
                permissions: vec![PermissionAction::List],
                caches: NameSelector::Name("foobar".to_string()),
                items: ItemSelector::All
            }],
            permissions.rules.expect("should have rules")
        );

        // Send request completely without conditions field
        // (Let API decide whether that preserves or removes conditions)
        assert_eq!(None, permissions.conditions);
    }

    #[test]
    fn test_determine_role_permissions_with_empty_conditions() {
        let permissions = determine_role_permissions(
            r#"{
                "rules": [
                    {
                        "type": "cache",
                        "permissions": [
                            "list"
                        ],
                        "caches": { "name": "foobar" },
                        "items": "*"
                    }
                ],
                "conditions": []
            }"#
            .to_string(),
        )
        .expect("should parse permissions");

        assert_eq!(
            vec![Rule::Cache {
                permissions: vec![PermissionAction::List],
                caches: NameSelector::Name("foobar".to_string()),
                items: ItemSelector::All
            }],
            permissions.rules.expect("should have rules")
        );

        // Send request with empty conditions field
        // (so `update` will remove all conditions)
        let conditions = permissions.conditions.expect("should have conditions");
        assert!(conditions.is_empty());
    }

    #[test]
    fn test_determine_role_permissions_with_no_rules() {
        let result = determine_role_permissions(
            r#"{
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
            }"#
            .to_string(),
        );

        // Require rules field with friendly error, because API requires it
        assert_matches!(
            result.expect_err("should reject permissions without rules"),
            CliError { msg, .. } if msg.contains("rules")
        );
    }

    #[test]
    fn test_determine_role_permissions_with_empty_rules() {
        let permissions = determine_role_permissions(
            r#"{
                "rules": [],
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
            }"#
            .to_string(),
        )
        .expect("should parse permissions");

        assert_eq!(
            Condition::IpFilter {
                allowed_cidr_ranges: vec!["10.1.2.3/32".to_string(), "5.4.3.2/24".to_string()]
            },
            permissions.conditions.expect("should have conditions")[0]
        );

        // Send request with empty rules field
        // (so `update` will remove all rules)
        let rules = permissions.rules.expect("should have rules");
        assert!(rules.is_empty());
    }

    // Rule Deserialization //

    #[test]
    fn test_deserialize_account_management_rule() {
        let rule = parse_permission_rule(
            r#"{
                "type": "account_management",
                "permissions": [
                    "read",
                    "list"
                ]
            }"#,
        );

        assert_eq!(
            Rule::AccountManagement {
                permissions: vec![PermissionAction::Read, PermissionAction::List],
            },
            rule
        );
    }

    #[test]
    fn test_deserialize_auth_management_rule() {
        let rule = parse_permission_rule(
            r#"{
                "type": "auth_management",
                "permissions": [
                    "read",
                    "write",
                    "list"
                ],
                "items": "*"
            }"#,
        );

        assert_eq!(
            Rule::AuthManagement {
                permissions: vec![
                    PermissionAction::Read,
                    PermissionAction::Write,
                    PermissionAction::List
                ],
                items: AllSelector::All,
            },
            rule
        );
    }

    #[test]
    fn test_deserialize_resource_management_rule() {
        let rule = parse_permission_rule(
            r#"{
                "type": "resource_management",
                "permissions": [
                    "read",
                    "write",
                    "list"
                ],
                "resources": "*"
            }"#,
        );

        assert_eq!(
            Rule::ResourceManagement {
                permissions: vec![
                    PermissionAction::Read,
                    PermissionAction::Write,
                    PermissionAction::List
                ],
                resources: AllSelector::All,
            },
            rule
        );
    }

    #[test]
    fn test_deserialize_database_rule() {
        let rule = parse_permission_rule(
            r#"{
                "type": "database",
                "permissions": [
                    "read",
                    "write"
                ],
                "databases": {
                    "name": "orders"
                },
                "items": {
                    "key_prefix": "orders:2026-"
                }
            }"#,
        );

        assert_eq!(
            Rule::Database {
                permissions: vec![PermissionAction::Read, PermissionAction::Write],
                databases: NameSelector::Name("orders".to_string()),
                items: ItemSelector::KeyPrefix("orders:2026-".to_string()),
            },
            rule
        );
    }

    #[test]
    fn test_deserialize_cache_rule() {
        let rule = parse_permission_rule(
            r#"{
                "type": "cache",
                "permissions": [
                    "read",
                    "write",
                    "list"
                ],
                "caches": "*",
                "items": {
                    "key_prefix": "public/"
                }
            }"#,
        );

        assert_eq!(
            Rule::Cache {
                permissions: vec![
                    PermissionAction::Read,
                    PermissionAction::Write,
                    PermissionAction::List
                ],
                caches: NameSelector::All,
                items: ItemSelector::KeyPrefix("public/".to_string()),
            },
            rule
        );
    }

    #[test]
    fn test_deserialize_topic_rule() {
        let rule = parse_permission_rule(
            r#"{
                "type": "topic",
                "permissions": [
                    "write"
                ],
                "caches": "*",
                "topics": {
                    "prefix": "room-"
                }
            }"#,
        );

        assert_eq!(
            Rule::Topic {
                permissions: vec![PermissionAction::Write],
                caches: NameSelector::All,
                topics: PrefixSelector::Prefix("room-".to_string()),
            },
            rule
        );
    }

    #[test]
    fn test_deserialize_store_rule() {
        let rule = parse_permission_rule(
            r#"{
                "type": "store",
                "permissions": [
                    "read",
                    "write",
                    "list"
                ],
                "stores": {
                    "name": "user-prefs"
                },
                "items": {
                    "key_prefix": "org:42:"
                }
            }"#,
        );

        assert_eq!(
            Rule::Store {
                permissions: vec![
                    PermissionAction::Read,
                    PermissionAction::Write,
                    PermissionAction::List
                ],
                stores: NameSelector::Name("user-prefs".to_string()),
                items: ItemSelector::KeyPrefix("org:42:".to_string()),
            },
            rule
        );
    }

    #[test]
    fn test_deserialize_function_rule() {
        let rule = parse_permission_rule(
            r#"{
                "type": "function",
                "permissions": [
                    "invoke"
                ],
                "caches": {
                    "name": "edge-app"
                },
                "functions": {
                    "prefix": "webhook-"
                }
            }"#,
        );

        assert_eq!(
            Rule::Function {
                permissions: vec![PermissionAction::Invoke],
                caches: NameSelector::Name("edge-app".to_string()),
                functions: PrefixSelector::Prefix("webhook-".to_string()),
            },
            rule
        );
    }

    #[test]
    fn test_deserialize_rule_with_invalid_fields() {
        serde_json::from_str::<Rule>(
            r#"{
                "type": "resource_management",
                "permissions": [
                    "read",
                    "write"
                    "list"
                ]
            }"#,
        )
        .expect_err("should require resources for resource_management");
    }

    #[test]
    /// https://docs.momentohq.com/platform/authentication/roles-http-api#full-permission-set-example
    fn test_deserialize_full_permissions_sample() {
        let result = serde_json::from_str::<Permissions>(
            r#"{
                "rules": [
                    { "type": "account_management",  "permissions": ["read", "list"] },
                    { "type": "auth_management",     "permissions": ["read", "write", "list"], "items": "*" },
                    { "type": "resource_management", "permissions": ["read", "write", "list"], "resources": "*" },
                    { "type": "database", "permissions": ["read", "write"],         "databases": "*",                   "items": "*" },
                    { "type": "database", "permissions": ["read"],                  "databases": { "name": "orders" },  "items": { "key_prefix": "orders:2026-" } },
                    { "type": "database", "permissions": ["write"],                 "databases": { "name": "orders" },  "items": { "key": "orders:pending" } },
                    { "type": "cache",    "permissions": ["read", "write", "list"], "caches": "*",                      "items": "*" },
                    { "type": "cache",    "permissions": ["read"],                  "caches": { "name": "prod-cache" }, "items": { "key_prefix": "public/" } },
                    { "type": "cache",    "permissions": ["write"],                 "caches": { "name": "prod-cache" }, "items": { "key": "feature-flags" } },
                    { "type": "topic",    "permissions": ["read", "write", "list"], "caches": "*",                      "topics": "*" },
                    { "type": "topic",    "permissions": ["read"],                  "caches": { "name": "chat-app" },   "topics": { "name": "announcements" } },
                    { "type": "topic",    "permissions": ["write"],                 "caches": { "name": "chat-app" },   "topics": { "prefix": "room-" } },
                    { "type": "store",    "permissions": ["read", "write", "list"], "stores": "*",                      "items": "*" },
                    { "type": "store",    "permissions": ["read"],                  "stores": { "name": "user-prefs" }, "items": { "key_prefix": "org:42:" } },
                    { "type": "store",    "permissions": ["write"],                 "stores": { "name": "user-prefs" }, "items": { "key": "schema-version" } },
                    { "type": "function", "permissions": ["invoke"],                "caches": "*",                      "functions": "*" },
                    { "type": "function", "permissions": ["invoke"],                "caches": { "name": "edge-app" },   "functions": { "name": "resize-image" } },
                    { "type": "function", "permissions": ["invoke"],                "caches": { "name": "edge-app" },   "functions": { "prefix": "webhook-" } }
                ],
                "conditions": [
                    { "ip_filter": { "allowed_cidr_ranges": ["10.0.0.0/8", "192.168.1.0/24", "2001:db8::/32"] } }
                ]
            }"#,
        );

        assert!(
            result.is_ok(),
            "should parse full permission set example from HTTP API docs"
        );
    }

    // Response Deserialization //

    #[test]
    fn test_deserialize_role_with_all_fields() {
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
        assert_eq!("custom", role.role_type);
        assert_eq!(
            "role with limited permissions",
            role.description.expect("should have description")
        );

        let rules = role.permissions.rules.expect("should have rules");
        assert_eq!(7, rules.len());

        let conditions = role.permissions.conditions.expect("should have conditions");
        assert_eq!(1, conditions.len());

        // Rules:
        assert_eq!(
            Rule::ResourceManagement {
                permissions: vec![PermissionAction::Read, PermissionAction::List],
                resources: AllSelector::All,
            },
            rules[0]
        );
        assert_eq!(
            Rule::Cache {
                permissions: vec![PermissionAction::List],
                caches: NameSelector::Name("foobar".to_string()),
                items: ItemSelector::All
            },
            rules[1]
        );
        assert_eq!(
            Rule::Cache {
                permissions: vec![PermissionAction::Read],
                caches: NameSelector::Name("foobar".to_string()),
                items: ItemSelector::KeyPrefix("hello".to_string())
            },
            rules[2]
        );
        assert_eq!(
            Rule::Cache {
                permissions: vec![PermissionAction::Write],
                caches: NameSelector::Name("foobar".to_string()),
                items: ItemSelector::Key("helloworld".to_string())
            },
            rules[3]
        );

        assert_eq!(
            Rule::Topic {
                permissions: vec![PermissionAction::Read, PermissionAction::List,],
                caches: NameSelector::Name("foobar".to_string()),
                topics: PrefixSelector::Prefix("prod-".to_string())
            },
            rules[4]
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
            rules[5]
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
            rules[6]
        );

        // Conditions:
        assert_eq!(
            Condition::IpFilter {
                allowed_cidr_ranges: vec!["10.1.2.3/32".to_string(), "5.4.3.2/24".to_string()]
            },
            conditions[0]
        );
    }

    #[test]
    fn test_deserialize_role_with_super_user_permissions() {
        let role = parse_role(
            r#"{
                "role_id": "r-owner",
                "role_name": "Owner",
                "description": "superuser role",
                "permissions": {
                    "super_user": true
                },
                "role_type": "system"
            }"#,
        );

        assert_eq!("Owner", role.name);
        assert_eq!("r-owner", role.id);
        assert_eq!("system", role.role_type);
        assert_eq!(
            "superuser role",
            role.description.expect("should have description")
        );

        let permissions = role.permissions;
        assert!(permissions.rules.is_none());
        assert!(permissions.conditions.is_none());
        assert!(permissions
            .super_user
            .expect("should have super_user field"));
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
                            "type": "cache",
                            "permissions": [
                                "list"
                            ],
                            "caches": { "name": "foobar" },
                            "items": "*"
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
        assert_eq!("custom", role.role_type);
        assert_eq!(
            vec![Rule::Cache {
                permissions: vec![PermissionAction::List],
                caches: NameSelector::Name("foobar".to_string()),
                items: ItemSelector::All
            }],
            role.permissions.rules.expect("should have rules")
        );
        assert_eq!(
            Condition::IpFilter {
                allowed_cidr_ranges: vec!["10.1.2.3/32".to_string(), "5.4.3.2/24".to_string()]
            },
            role.permissions.conditions.expect("should have conditions")[0]
        );

        assert_eq!(None, role.description);
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
                            "type": "cache",
                            "permissions": [
                                "list"
                            ],
                            "caches": { "name": "foobar" },
                            "items": "*"
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
        assert_eq!("custom", role.role_type);
        assert_eq!(
            vec![Rule::Cache {
                permissions: vec![PermissionAction::List],
                caches: NameSelector::Name("foobar".to_string()),
                items: ItemSelector::All
            }],
            role.permissions.rules.expect("should have rules")
        );
        assert_eq!(
            Condition::IpFilter {
                allowed_cidr_ranges: vec!["10.1.2.3/32".to_string(), "5.4.3.2/24".to_string()]
            },
            role.permissions.conditions.expect("should have conditions")[0]
        );

        assert_eq!("", role.description.expect("should have description"));
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
        assert_eq!("custom", role.role_type);
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
            role.permissions.rules.expect("should have rules")
        );

        assert!(role.permissions.conditions.is_none());
    }

    #[test]
    fn test_deserialize_role_with_no_rules() {
        let role = parse_role(
            r#"{
                "role_id": "r-limited",
                "role_name": "Limited",
                "description": "role with limited permissions",
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
        assert_eq!("custom", role.role_type);
        assert_eq!(
            "role with limited permissions",
            role.description.expect("should have description")
        );
        assert_eq!(
            Condition::IpFilter {
                allowed_cidr_ranges: vec!["10.1.2.3/32".to_string(), "5.4.3.2/24".to_string()]
            },
            role.permissions.conditions.expect("should have conditions")[0]
        );

        assert!(role.permissions.rules.is_none());
    }
}
