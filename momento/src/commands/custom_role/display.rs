use super::utils::{
    AccountMember, ActiveReferences, ApiKey, Condition, CustomRoleResponse, Invitation,
    ItemSelector, NameSelector, PrefixSelector, Rule,
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
                        NameSelector::All => "Caches (all)".to_string(),
                        NameSelector::Name(name) => format!("Cache: {name}"),
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
                        NameSelector::All => "In caches: all".to_string(),
                        NameSelector::Name(name) => format!("In cache: {name}"),
                    },
                ),
                Rule::Store {
                    permissions: _,
                    stores,
                    items,
                } => format!(
                    "{}\n  {}\n",
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
                        NameSelector::All => "In caches: all".to_string(),
                        NameSelector::Name(name) => format!("In cache: {name}"),
                    },
                ),
                Rule::Database {
                    permissions: _,
                    databases,
                } => match databases {
                    NameSelector::All => "Databases (all)".to_string(),
                    NameSelector::Name(name) => format!("Database: {name}"),
                },
                Rule::AccountManagement { permissions: _ } => "Account Management:".to_string(),
                Rule::AuthManagement { permissions: _ } => "Auth Management:".to_string(),
                Rule::ResourceManagement { permissions: _ } => "Resource Management:".to_string(),
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

impl fmt::Display for CustomRoleResponse {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Name: {}", self.name)?;
        write!(f, "\nID: {}", self.id)?;
        if let Some(description) = &self.description {
            write!(f, "\nDescription: {description}")?;
        }
        if !self.permissions.rules.is_empty() {
            write!(f, "\nPermissions:")?;
            for rule in &self.permissions.rules {
                write!(f, "\n{rule}")?;
            }
        } else {
            write!(f, "\nPermissions: (none)")?;
        }
        if !self.permissions.conditions.is_empty() {
            write!(f, "\nConditions:")?;
            for condition in &self.permissions.conditions {
                write!(f, "\n{condition}")?;
            }
        } else {
            write!(f, "\nConditions: (none)")?;
        }
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
    use super::super::utils::{ItemSelector, NameSelector};
    use super::super::utils::{PermissionAction, Permissions, Rule};
    use super::*;

    fn snapshot_settings() -> insta::Settings {
        let mut settings = insta::Settings::clone_current();
        settings.set_prepend_module_to_snapshot(false);
        settings
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
    fn test_display_role_with_no_description() {
        let role = CustomRoleResponse {
            id: "r-limited".to_string(),
            name: "Limited".to_string(),
            description: None,
            permissions: Permissions {
                rules: vec![
                    Rule::ResourceManagement {
                        permissions: vec![PermissionAction::Read, PermissionAction::List],
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
                ],
                conditions: vec![Condition::IpFilter {
                    allowed_cidr_ranges: vec!["10.1.2.3/32".to_string(), "5.4.3.2/24".to_string()],
                }],
            },
        };

        snapshot_settings().bind(|| insta::assert_snapshot!(role.to_string()));
    }

    #[test]
    fn test_display_role_with_empty_description() {
        let role = CustomRoleResponse {
            id: "r-limited".to_string(),
            name: "Limited".to_string(),
            description: Some("".to_string()),
            permissions: Permissions {
                rules: vec![
                    Rule::ResourceManagement {
                        permissions: vec![PermissionAction::Read, PermissionAction::List],
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
                ],
                conditions: vec![Condition::IpFilter {
                    allowed_cidr_ranges: vec!["10.1.2.3/32".to_string(), "5.4.3.2/24".to_string()],
                }],
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
                                    "0.0.0.0/0"
                                ]
                            }
                        }
                    ]
                },
                "role_type": "custom"
            }"#,
        )
        .expect("should parse a custom role");

        snapshot_settings().bind(|| insta::assert_snapshot!(role.to_string()));
    }
}
