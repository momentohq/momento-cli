use super::utils::{
    AccountMember, ActiveReferences, AllSelector, ApiKey, Condition, CustomRoleResponse,
    Invitation, ItemSelector, NameSelector, Permissions, PrefixSelector, Rule,
};

use chrono::prelude::DateTime;
use std::fmt;

impl Rule {
    fn format_permissions(&self) -> String {
        match self {
            Rule::Cache { permissions, .. } => permissions,
            Rule::Topic { permissions, .. } => permissions,
            Rule::Store { permissions, .. } => permissions,
            Rule::Function { permissions, .. } => permissions,
            Rule::Database { permissions, .. } => permissions,
            Rule::AccountManagement { permissions, .. } => permissions,
            Rule::AuthManagement { permissions, .. } => permissions,
            Rule::ResourceManagement { permissions, .. } => permissions,
        }
        .iter()
        .map(|permission| format!("{permission:?}"))
        .collect::<Vec<String>>()
        .join(", ")
    }
}

impl fmt::Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "- {}\n  Allowed actions: {}",
            match self {
                Rule::Cache {
                    permissions: _,
                    caches,
                    items,
                } => format!(
                    "{}\n  {}",
                    match caches {
                        NameSelector::All => "Serverless Caches (all)".to_string(),
                        NameSelector::Name(name) => format!("Serverless Cache: {name}"),
                    },
                    match items {
                        ItemSelector::All => "Keys: all".to_string(),
                        ItemSelector::Key(name) => format!("Key: {name}"),
                        ItemSelector::KeyPrefix(prefix) => format!("Keys with prefix: {prefix}"),
                    },
                ),
                Rule::Topic {
                    permissions: _,
                    topics,
                    caches,
                } => format!(
                    "{}\n  {}",
                    match topics {
                        PrefixSelector::All => "Topics (all)".to_string(),
                        PrefixSelector::Name(name) => format!("Topic: {name}"),
                        PrefixSelector::Prefix(prefix) => format!("Topics with prefix: {prefix}"),
                    },
                    match caches {
                        NameSelector::All => "In Serverless Caches: all".to_string(),
                        NameSelector::Name(name) => format!("In Serverless Cache: {name}"),
                    },
                ),
                Rule::Store {
                    permissions: _,
                    stores,
                    items,
                } => format!(
                    "{}\n  {}",
                    match stores {
                        NameSelector::All => "Object Stores (all)".to_string(),
                        NameSelector::Name(name) => format!("Object Store: {name}"),
                    },
                    match items {
                        ItemSelector::All => "Keys: all".to_string(),
                        ItemSelector::Key(name) => format!("Key: {name}"),
                        ItemSelector::KeyPrefix(prefix) => format!("Keys with prefix: {prefix}"),
                    },
                ),
                Rule::Function {
                    permissions: _,
                    functions,
                    caches,
                } => format!(
                    "{}\n  {}",
                    match functions {
                        PrefixSelector::All => "Functions (all)".to_string(),
                        PrefixSelector::Name(name) => format!("Function: {name}"),
                        PrefixSelector::Prefix(prefix) =>
                            format!("Functions with prefix: {prefix}"),
                    },
                    match caches {
                        NameSelector::All => "In Serverless Caches: all".to_string(),
                        NameSelector::Name(name) => format!("In Serverless Cache: {name}"),
                    },
                ),
                Rule::Database {
                    permissions: _,
                    databases,
                    items,
                } => format!(
                    "{}\n  {}",
                    match databases {
                        NameSelector::All => "Databases (all)".to_string(),
                        NameSelector::Name(name) => format!("Database: {name}"),
                    },
                    match items {
                        ItemSelector::All => "Keys: all".to_string(),
                        ItemSelector::Key(name) => format!("Key: {name}"),
                        ItemSelector::KeyPrefix(prefix) => format!("Keys with prefix: {prefix}"),
                    },
                ),
                Rule::AccountManagement { permissions: _ } => "Account Management:".to_string(),
                Rule::AuthManagement {
                    permissions: _,
                    items,
                } => match items {
                    AllSelector::All => "Auth Management:\n  Items: all".to_string(),
                },
                Rule::ResourceManagement {
                    permissions: _,
                    resources,
                } => match resources {
                    AllSelector::All => "Resource Management:\n  Resources: all".to_string(),
                },
            },
            self.format_permissions(),
        )
    }
}

impl fmt::Display for Condition {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "- {}",
            match self {
                Condition::IpFilter {
                    allowed_cidr_ranges,
                } => format!("Allowed IPs:\n  {}", allowed_cidr_ranges.join("\n  ")),
            }
        )
    }
}

impl fmt::Display for Permissions {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.super_user == Some(true) {
            write!(f, "Permissions: super user")?;
        } else {
            match &self.rules {
                Some(rules) if !rules.is_empty() => {
                    write!(f, "Rules:")?;
                    for rule in rules {
                        write!(f, "\n{rule}")?;
                    }
                }
                _ => write!(f, "Rules: (none)")?,
            }
            match &self.conditions {
                Some(conditions) if !conditions.is_empty() => {
                    write!(f, "\nConditions:")?;
                    for condition in conditions {
                        write!(f, "\n{condition}")?;
                    }
                }
                _ => write!(f, "\nConditions: (none)")?,
            }
        }
        Ok(())
    }
}

impl fmt::Display for CustomRoleResponse {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Name: {}", self.name)?;
        write!(
            f,
            "\nID: {}{}",
            self.id,
            if self.role_type == "system" {
                " (system role)"
            } else {
                ""
            }
        )?;
        if let Some(description) = &self.description {
            write!(f, "\nDescription: {description}")?;
        }
        write!(f, "\n{}", self.permissions)?;
        Ok(())
    }
}

/// delete_role
impl fmt::Display for AccountMember {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "- {}", self.user_name)?;
        Ok(())
    }
}

impl fmt::Display for Invitation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "- {}", self.account_member.user_name)?;
        Ok(())
    }
}

impl fmt::Display for ApiKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "- Key ID: {}", self.key_id)?;
        writeln!(f, "  Account ID: {}", self.account_id)?;
        writeln!(f, "  Description: {}", self.description)?;
        let issued_at = match DateTime::from_timestamp(self.issued_at_epoch_seconds, 0) {
            Some(datetime) => datetime.to_string(),
            None => format!("{} (epoch seconds)", self.issued_at_epoch_seconds),
        };
        write!(f, "  Issued At: {}", issued_at)?;
        Ok(())
    }
}

impl fmt::Display for ActiveReferences {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut refs = vec![];
        if !self.account_members.is_empty() {
            refs.push(format!(
                "Account Members:\n{}",
                self.account_members
                    .iter()
                    .map(|member| member.to_string())
                    .collect::<Vec<_>>()
                    .join("\n- ")
            ));
        }
        if !self.invitations.is_empty() {
            refs.push(format!(
                "Invited Account Members:\n{}",
                self.invitations
                    .iter()
                    .map(|invite| invite.to_string())
                    .collect::<Vec<_>>()
                    .join("\n- ")
            ));
        }
        if !self.api_keys.is_empty() {
            refs.push(format!(
                "API Keys:\n{}",
                self.api_keys
                    .iter()
                    .map(|key| key.to_string())
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }
        write!(f, "{}", refs.join("\n"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::utils::{AllSelector, ItemSelector, NameSelector};
    use super::super::utils::{PermissionAction, Permissions, Rule};
    use super::*;

    fn snapshot_settings() -> insta::Settings {
        let mut settings = insta::Settings::clone_current();
        settings.set_prepend_module_to_snapshot(false);
        settings
    }

    #[test]
    fn test_display_account_management_rule_with_all_permissions() {
        let rule = Rule::AccountManagement {
            permissions: vec![PermissionAction::Read, PermissionAction::List],
        };
        snapshot_settings().bind(|| insta::assert_snapshot!(rule.to_string()));
    }

    #[test]
    fn test_display_auth_management_rule_with_all_permissions() {
        let rule = Rule::AuthManagement {
            permissions: vec![
                PermissionAction::Read,
                PermissionAction::Write,
                PermissionAction::List,
            ],
            items: AllSelector::All,
        };
        snapshot_settings().bind(|| insta::assert_snapshot!(rule.to_string()));
    }

    #[test]
    fn test_display_resource_management_rule_with_all_permissions() {
        let rule = Rule::ResourceManagement {
            permissions: vec![
                PermissionAction::Read,
                PermissionAction::Write,
                PermissionAction::List,
            ],
            resources: AllSelector::All,
        };
        snapshot_settings().bind(|| insta::assert_snapshot!(rule.to_string()));
    }

    #[test]
    fn test_display_database_rule_with_all_permissions() {
        let rule = Rule::Database {
            permissions: vec![PermissionAction::Read, PermissionAction::Write],
            databases: NameSelector::All,
            items: ItemSelector::All,
        };
        snapshot_settings().bind(|| insta::assert_snapshot!(rule.to_string()));
    }

    #[test]
    fn test_display_database_rule_with_name_permissions() {
        let rule = Rule::Database {
            permissions: vec![PermissionAction::Read, PermissionAction::Write],
            databases: NameSelector::Name("orders".to_string()),
            items: ItemSelector::Key("orders:pending".to_string()),
        };
        snapshot_settings().bind(|| insta::assert_snapshot!(rule.to_string()));
    }

    #[test]
    fn test_display_cache_rule_with_all_permissions() {
        let rule = Rule::Cache {
            permissions: vec![
                PermissionAction::Read,
                PermissionAction::Write,
                PermissionAction::List,
            ],
            caches: NameSelector::All,
            items: ItemSelector::All,
        };
        snapshot_settings().bind(|| insta::assert_snapshot!(rule.to_string()));
    }

    #[test]
    fn test_display_cache_rule_with_limited_permissions_by_name() {
        let rule = Rule::Cache {
            permissions: vec![PermissionAction::Write],
            caches: NameSelector::Name("foobar".to_string()),
            items: ItemSelector::Key("helloworld".to_string()),
        };
        snapshot_settings().bind(|| insta::assert_snapshot!(rule.to_string()));
    }

    #[test]
    fn test_display_cache_rule_with_limited_permissions_by_prefix() {
        let rule = Rule::Cache {
            permissions: vec![PermissionAction::Write],
            caches: NameSelector::Name("foobar".to_string()),
            items: ItemSelector::KeyPrefix("hello".to_string()),
        };
        snapshot_settings().bind(|| insta::assert_snapshot!(rule.to_string()));
    }

    #[test]
    fn test_display_topic_rule_with_all_permissions() {
        let rule = Rule::Topic {
            permissions: vec![
                PermissionAction::Read,
                PermissionAction::Write,
                PermissionAction::List,
            ],
            caches: NameSelector::All,
            topics: PrefixSelector::All,
        };
        snapshot_settings().bind(|| insta::assert_snapshot!(rule.to_string()));
    }

    #[test]
    fn test_display_topic_rule_with_name_permissions() {
        let rule = Rule::Topic {
            permissions: vec![PermissionAction::Read],
            caches: NameSelector::Name("chat-app".to_string()),
            topics: PrefixSelector::Name("announcements".to_string()),
        };
        snapshot_settings().bind(|| insta::assert_snapshot!(rule.to_string()));
    }

    #[test]
    fn test_display_topic_rule_with_prefix_permissions() {
        let rule = Rule::Topic {
            permissions: vec![PermissionAction::Write],
            caches: NameSelector::All,
            topics: PrefixSelector::Prefix("room-".to_string()),
        };
        snapshot_settings().bind(|| insta::assert_snapshot!(rule.to_string()));
    }

    #[test]
    fn test_display_store_rule_with_all_permissions() {
        let rule = Rule::Store {
            permissions: vec![
                PermissionAction::Read,
                PermissionAction::Write,
                PermissionAction::List,
            ],
            stores: NameSelector::All,
            items: ItemSelector::All,
        };
        snapshot_settings().bind(|| insta::assert_snapshot!(rule.to_string()));
    }

    #[test]
    fn test_display_store_rule_with_name_permissions() {
        let rule = Rule::Store {
            permissions: vec![
                PermissionAction::Read,
                PermissionAction::Write,
                PermissionAction::List,
            ],
            stores: NameSelector::Name("user-prefs".to_string()),
            items: ItemSelector::Key("schema-version".to_string()),
        };
        snapshot_settings().bind(|| insta::assert_snapshot!(rule.to_string()));
    }

    #[test]
    fn test_display_store_rule_with_prefix_permissions() {
        let rule = Rule::Store {
            permissions: vec![
                PermissionAction::Read,
                PermissionAction::Write,
                PermissionAction::List,
            ],
            stores: NameSelector::All,
            items: ItemSelector::KeyPrefix("org:42:".to_string()),
        };
        snapshot_settings().bind(|| insta::assert_snapshot!(rule.to_string()));
    }

    #[test]
    fn test_display_function_rule_with_all_permissions() {
        let rule = Rule::Function {
            permissions: vec![PermissionAction::Invoke],
            caches: NameSelector::All,
            functions: PrefixSelector::All,
        };
        snapshot_settings().bind(|| insta::assert_snapshot!(rule.to_string()));
    }

    #[test]
    fn test_display_function_rule_with_name_permissions() {
        let rule = Rule::Function {
            permissions: vec![PermissionAction::Invoke],
            caches: NameSelector::Name("edge-app".to_string()),
            functions: PrefixSelector::Name("resize-image".to_string()),
        };
        snapshot_settings().bind(|| insta::assert_snapshot!(rule.to_string()));
    }

    #[test]
    fn test_display_function_rule_with_prefix_permissions() {
        let rule = Rule::Function {
            permissions: vec![PermissionAction::Invoke],
            caches: NameSelector::Name("edge-app".to_string()),
            functions: PrefixSelector::Prefix("webhook-".to_string()),
        };
        snapshot_settings().bind(|| insta::assert_snapshot!(rule.to_string()));
    }

    #[test]
    fn test_display_role_with_super_user_permissions() {
        let role = CustomRoleResponse {
            id: "r-owner".to_string(),
            name: "Owner".to_string(),
            role_type: "system".to_string(),
            description: Some("superuser role".to_string()),
            permissions: Permissions {
                super_user: Some(true),
                rules: None,
                conditions: None,
            },
        };

        snapshot_settings().bind(|| insta::assert_snapshot!(role.to_string()));
    }

    #[test]
    fn test_display_role_with_no_description() {
        let role = CustomRoleResponse {
            id: "r-limited".to_string(),
            name: "Limited".to_string(),
            role_type: "custom".to_string(),
            description: None,
            permissions: Permissions {
                super_user: None,
                rules: Some(vec![
                    Rule::ResourceManagement {
                        permissions: vec![PermissionAction::Read, PermissionAction::List],
                        resources: AllSelector::All,
                    },
                    Rule::Cache {
                        permissions: vec![PermissionAction::List],
                        caches: NameSelector::Name("foobar".to_string()),
                        items: ItemSelector::All,
                    },
                    Rule::Cache {
                        permissions: vec![PermissionAction::Read],
                        caches: NameSelector::Name("foobar".to_string()),
                        items: ItemSelector::KeyPrefix("hello".to_string()),
                    },
                    Rule::Cache {
                        permissions: vec![PermissionAction::Write],
                        caches: NameSelector::Name("foobar".to_string()),
                        items: ItemSelector::Key("helloworld".to_string()),
                    },
                    Rule::Topic {
                        permissions: vec![PermissionAction::Read, PermissionAction::List],
                        caches: NameSelector::Name("foobar".to_string()),
                        topics: PrefixSelector::Prefix("prod-".to_string()),
                    },
                    Rule::Topic {
                        permissions: vec![
                            PermissionAction::Read,
                            PermissionAction::List,
                            PermissionAction::Write,
                        ],
                        caches: NameSelector::Name("foobar".to_string()),
                        topics: PrefixSelector::Prefix("preprod-".to_string()),
                    },
                    Rule::Topic {
                        permissions: vec![
                            PermissionAction::Read,
                            PermissionAction::List,
                            PermissionAction::Write,
                        ],
                        caches: NameSelector::All,
                        topics: PrefixSelector::Name("dev".to_string()),
                    },
                ]),
                conditions: Some(vec![Condition::IpFilter {
                    allowed_cidr_ranges: vec!["10.1.2.3/32".to_string(), "5.4.3.2/24".to_string()],
                }]),
            },
        };

        snapshot_settings().bind(|| insta::assert_snapshot!(role.to_string()));
    }

    #[test]
    fn test_display_role_with_empty_description() {
        let role = CustomRoleResponse {
            id: "r-limited".to_string(),
            name: "Limited".to_string(),
            role_type: "custom".to_string(),
            description: Some("".to_string()),
            permissions: Permissions {
                super_user: None,
                rules: Some(vec![
                    Rule::ResourceManagement {
                        permissions: vec![PermissionAction::Read, PermissionAction::List],
                        resources: AllSelector::All,
                    },
                    Rule::Cache {
                        permissions: vec![PermissionAction::List],
                        caches: NameSelector::Name("foobar".to_string()),
                        items: ItemSelector::All,
                    },
                    Rule::Cache {
                        permissions: vec![PermissionAction::Read],
                        caches: NameSelector::Name("foobar".to_string()),
                        items: ItemSelector::KeyPrefix("hello".to_string()),
                    },
                    Rule::Cache {
                        permissions: vec![PermissionAction::Write],
                        caches: NameSelector::Name("foobar".to_string()),
                        items: ItemSelector::Key("helloworld".to_string()),
                    },
                    Rule::Topic {
                        permissions: vec![PermissionAction::Read, PermissionAction::List],
                        caches: NameSelector::Name("foobar".to_string()),
                        topics: PrefixSelector::Prefix("prod-".to_string()),
                    },
                    Rule::Topic {
                        permissions: vec![
                            PermissionAction::Read,
                            PermissionAction::List,
                            PermissionAction::Write,
                        ],
                        caches: NameSelector::Name("foobar".to_string()),
                        topics: PrefixSelector::Prefix("preprod-".to_string()),
                    },
                    Rule::Topic {
                        permissions: vec![
                            PermissionAction::Read,
                            PermissionAction::List,
                            PermissionAction::Write,
                        ],
                        caches: NameSelector::All,
                        topics: PrefixSelector::Name("dev".to_string()),
                    },
                ]),
                conditions: Some(vec![Condition::IpFilter {
                    allowed_cidr_ranges: vec!["10.1.2.3/32".to_string(), "5.4.3.2/24".to_string()],
                }]),
            },
        };

        snapshot_settings().bind(|| insta::assert_snapshot!(role.to_string()));
    }

    #[test]
    fn test_display_role_with_no_conditions() {
        let role = CustomRoleResponse {
            id: "r-limited".to_string(),
            name: "Limited".to_string(),
            role_type: "custom".to_string(),
            description: Some("role with limited permissions".to_string()),
            permissions: Permissions {
                super_user: None,
                rules: Some(vec![
                    Rule::ResourceManagement {
                        permissions: vec![PermissionAction::Read, PermissionAction::List],
                        resources: AllSelector::All,
                    },
                    Rule::Cache {
                        permissions: vec![PermissionAction::List],
                        caches: NameSelector::Name("foobar".to_string()),
                        items: ItemSelector::All,
                    },
                    Rule::Cache {
                        permissions: vec![PermissionAction::Read],
                        caches: NameSelector::Name("foobar".to_string()),
                        items: ItemSelector::KeyPrefix("hello".to_string()),
                    },
                    Rule::Cache {
                        permissions: vec![PermissionAction::Write],
                        caches: NameSelector::Name("foobar".to_string()),
                        items: ItemSelector::Key("helloworld".to_string()),
                    },
                    Rule::Topic {
                        permissions: vec![PermissionAction::Read, PermissionAction::List],
                        caches: NameSelector::Name("foobar".to_string()),
                        topics: PrefixSelector::Prefix("prod-".to_string()),
                    },
                    Rule::Topic {
                        permissions: vec![
                            PermissionAction::Read,
                            PermissionAction::List,
                            PermissionAction::Write,
                        ],
                        caches: NameSelector::Name("foobar".to_string()),
                        topics: PrefixSelector::Prefix("preprod-".to_string()),
                    },
                    Rule::Topic {
                        permissions: vec![
                            PermissionAction::Read,
                            PermissionAction::List,
                            PermissionAction::Write,
                        ],
                        caches: NameSelector::All,
                        topics: PrefixSelector::Name("dev".to_string()),
                    },
                ]),
                conditions: None,
            },
        };

        snapshot_settings().bind(|| insta::assert_snapshot!(role.to_string()));
    }

    #[test]
    fn test_display_role_with_empty_conditions() {
        let role = CustomRoleResponse {
            id: "r-limited".to_string(),
            name: "Limited".to_string(),
            role_type: "custom".to_string(),
            description: Some("role with limited permissions".to_string()),
            permissions: Permissions {
                super_user: None,
                rules: Some(vec![
                    Rule::ResourceManagement {
                        permissions: vec![PermissionAction::Read, PermissionAction::List],
                        resources: AllSelector::All,
                    },
                    Rule::Cache {
                        permissions: vec![PermissionAction::List],
                        caches: NameSelector::Name("foobar".to_string()),
                        items: ItemSelector::All,
                    },
                    Rule::Cache {
                        permissions: vec![PermissionAction::Read],
                        caches: NameSelector::Name("foobar".to_string()),
                        items: ItemSelector::KeyPrefix("hello".to_string()),
                    },
                    Rule::Cache {
                        permissions: vec![PermissionAction::Write],
                        caches: NameSelector::Name("foobar".to_string()),
                        items: ItemSelector::Key("helloworld".to_string()),
                    },
                    Rule::Topic {
                        permissions: vec![PermissionAction::Read, PermissionAction::List],
                        caches: NameSelector::Name("foobar".to_string()),
                        topics: PrefixSelector::Prefix("prod-".to_string()),
                    },
                    Rule::Topic {
                        permissions: vec![
                            PermissionAction::Read,
                            PermissionAction::List,
                            PermissionAction::Write,
                        ],
                        caches: NameSelector::Name("foobar".to_string()),
                        topics: PrefixSelector::Prefix("preprod-".to_string()),
                    },
                    Rule::Topic {
                        permissions: vec![
                            PermissionAction::Read,
                            PermissionAction::List,
                            PermissionAction::Write,
                        ],
                        caches: NameSelector::All,
                        topics: PrefixSelector::Name("dev".to_string()),
                    },
                ]),
                conditions: Some(vec![]),
            },
        };

        snapshot_settings().bind(|| insta::assert_snapshot!(role.to_string()));
    }

    #[test]
    fn test_display_role_with_empty_rules() {
        let role = CustomRoleResponse {
            id: "r-limited".to_string(),
            name: "Limited".to_string(),
            role_type: "custom".to_string(),
            description: Some("role with limited permissions".to_string()),
            permissions: Permissions {
                super_user: None,
                rules: Some(vec![]),
                conditions: Some(vec![Condition::IpFilter {
                    allowed_cidr_ranges: vec!["10.1.2.3/32".to_string(), "5.4.3.2/24".to_string()],
                }]),
            },
        };

        snapshot_settings().bind(|| insta::assert_snapshot!(role.to_string()));
    }

    #[test]
    fn test_deserialize_through_display() {
        let role: CustomRoleResponse = serde_json::from_str(
            r#"{
                "role_id": "r-limited",
                "role_name": "Limited",
                "description": "I have a description",
                "permissions": {
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
                },
                "role_type": "custom"
            }"#,
        )
        .expect("should parse a custom role");

        snapshot_settings().bind(|| insta::assert_snapshot!(role.to_string()));
    }
}
