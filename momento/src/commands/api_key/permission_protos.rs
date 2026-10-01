//! A token's embedded permissions, translated into the role model
//! so they print the same way `momento role` prints a role.

use crate::commands::custom_role::utils::{
    AllSelector, Condition, ItemSelector, NameSelector, PermissionAction, Permissions,
    PrefixSelector, Rule,
};
use crate::error::CliError;

use momento_protos::{permission_messages as v1, permission_rules as v2};
use prost::UnknownEnumValue;
pub use v1::Permissions as PermissionsProtoV1;
pub use v2::PermissionSet as PermissionsProtoV2;

fn super_user() -> Permissions {
    Permissions {
        super_user: Some(true),
        rules: None,
        conditions: None,
    }
}

fn required<T>(value: Option<T>, what: &str) -> Result<T, CliError> {
    value.ok_or_else(|| CliError::new(format!("unrecognized or missing {what}")))
}

fn actions<E: TryFrom<i32, Error = UnknownEnumValue>>(
    values: &[i32],
    action: fn(E) -> PermissionAction,
) -> Result<Vec<PermissionAction>, CliError> {
    values
        .iter()
        .map(|&value| E::try_from(value).map(action).map_err(unknown))
        .collect()
}

fn unknown(error: UnknownEnumValue) -> CliError {
    CliError::new(format!("unrecognized permission value {}", error.0))
}

/// Keys are bytes; show them as text, which is what they almost always are.
fn key(bytes: Vec<u8>) -> String {
    String::from_utf8_lossy(&bytes).into_owned()
}

fn v1_rule(permission: v1::PermissionsType) -> Result<Rule, CliError> {
    use v1::permissions_type::{
        cache_item_selector, cache_permissions, function_permissions, function_selector,
        topic_permissions, topic_selector, Kind,
    };
    use PermissionAction::{Invoke, Read, Write};

    Ok(match required(permission.kind, "permission kind")? {
        Kind::CachePermissions(p) => Rule::Cache {
            permissions: match v1::CacheRole::try_from(p.role).map_err(unknown)? {
                v1::CacheRole::CachePermitNone => vec![],
                v1::CacheRole::CacheReadWrite => vec![Read, Write],
                v1::CacheRole::CacheReadOnly => vec![Read],
                v1::CacheRole::CacheWriteOnly => vec![Write],
            },
            caches: match required(p.cache, "cache selector")? {
                cache_permissions::Cache::AllCaches(_) => NameSelector::All,
                cache_permissions::Cache::CacheSelector(s) => v1_cache(s)?,
            },
            // Unset means every item: that is how the SDK writes a whole-cache permission.
            items: match p.cache_item {
                None | Some(cache_permissions::CacheItem::AllItems(_)) => ItemSelector::All,
                Some(cache_permissions::CacheItem::ItemSelector(s)) => {
                    match required(s.kind, "item selector")? {
                        cache_item_selector::Kind::Key(k) => ItemSelector::Key(key(k)),
                        cache_item_selector::Kind::KeyPrefix(k) => ItemSelector::KeyPrefix(key(k)),
                    }
                }
            },
        },
        Kind::TopicPermissions(p) => Rule::Topic {
            permissions: match v1::TopicRole::try_from(p.role).map_err(unknown)? {
                v1::TopicRole::TopicPermitNone => vec![],
                v1::TopicRole::TopicReadWrite => vec![Read, Write],
                v1::TopicRole::TopicReadOnly => vec![Read],
                v1::TopicRole::TopicWriteOnly => vec![Write],
            },
            topics: match required(p.topic, "topic selector")? {
                topic_permissions::Topic::AllTopics(_) => PrefixSelector::All,
                topic_permissions::Topic::TopicSelector(s) => {
                    match required(s.kind, "topic selector")? {
                        topic_selector::Kind::TopicName(n) => PrefixSelector::Name(n),
                        topic_selector::Kind::TopicNamePrefix(n) => PrefixSelector::Prefix(n),
                    }
                }
            },
            caches: match required(p.cache, "cache selector")? {
                topic_permissions::Cache::AllCaches(_) => NameSelector::All,
                topic_permissions::Cache::CacheSelector(s) => v1_cache(s)?,
            },
        },
        Kind::FunctionPermissions(p) => Rule::Function {
            permissions: match v1::FunctionRole::try_from(p.role).map_err(unknown)? {
                v1::FunctionRole::FunctionPermitNone => vec![],
                v1::FunctionRole::FunctionInvoke => vec![Invoke],
            },
            functions: match required(p.function, "function selector")? {
                function_permissions::Function::AllFunctions(_) => PrefixSelector::All,
                function_permissions::Function::FunctionSelector(s) => {
                    match required(s.kind, "function selector")? {
                        function_selector::Kind::FunctionName(n) => PrefixSelector::Name(n),
                        function_selector::Kind::FunctionNamePrefix(n) => PrefixSelector::Prefix(n),
                    }
                }
            },
            caches: match required(p.cache, "cache selector")? {
                function_permissions::Cache::AllCaches(_) => NameSelector::All,
                function_permissions::Cache::CacheSelector(s) => v1_cache(s)?,
            },
        },
    })
}

fn v1_cache(selector: v1::permissions_type::CacheSelector) -> Result<NameSelector, CliError> {
    match required(selector.kind, "cache selector")? {
        v1::permissions_type::cache_selector::Kind::CacheName(name) => Ok(NameSelector::Name(name)),
    }
}

fn v2_rule(rule: v2::Rule) -> Result<Rule, CliError> {
    use v2::rule::{
        auth_management_rule, cache_rule, database_rule, function_rule, resource_management_rule,
        store_rule, topic_rule, Kind,
    };
    use PermissionAction::{Invoke, List, Read, Write};

    Ok(match required(rule.kind, "rule kind")? {
        Kind::AccountManagementRule(r) => Rule::AccountManagement {
            permissions: actions(&r.permissions, |p| match p {
                v2::AccountManagementPermissions::AccountRead => Read,
                v2::AccountManagementPermissions::AccountList => List,
            })?,
        },
        Kind::AuthManagementRule(r) => Rule::AuthManagement {
            permissions: actions(&r.permissions, |p| match p {
                v2::AuthManagementPermissions::AuthRead => Read,
                v2::AuthManagementPermissions::AuthWrite => Write,
                v2::AuthManagementPermissions::AuthList => List,
            })?,
            items: match r.auth_items {
                None | Some(auth_management_rule::AuthItems::AllItems(_)) => AllSelector::All,
            },
        },
        Kind::ResourceManagementRule(r) => Rule::ResourceManagement {
            permissions: actions(&r.permissions, |p| match p {
                v2::ResourceManagementPermissions::ResourceRead => Read,
                v2::ResourceManagementPermissions::ResourceWrite => Write,
                v2::ResourceManagementPermissions::ResourceList => List,
            })?,
            resources: match r.resources {
                None | Some(resource_management_rule::Resources::AllResources(_)) => {
                    AllSelector::All
                }
            },
        },
        Kind::CacheRule(r) => Rule::Cache {
            permissions: actions(&r.permissions, |p| match p {
                v2::CacheApiPermissions::CacheList => List,
                v2::CacheApiPermissions::CacheRead => Read,
                v2::CacheApiPermissions::CacheWrite => Write,
            })?,
            caches: match required(r.cache, "cache selector")? {
                cache_rule::Cache::AllCaches(_) => NameSelector::All,
                cache_rule::Cache::CacheSelector(s) => v2_cache(s)?,
            },
            items: match r.cache_item {
                None | Some(cache_rule::CacheItem::AllItems(_)) => ItemSelector::All,
                Some(cache_rule::CacheItem::ItemSelector(s)) => {
                    match required(s.kind, "item selector")? {
                        v2::cache_item_selector::Kind::Key(k) => ItemSelector::Key(key(k)),
                        v2::cache_item_selector::Kind::KeyPrefix(k) => {
                            ItemSelector::KeyPrefix(key(k))
                        }
                    }
                }
            },
        },
        Kind::TopicRule(r) => Rule::Topic {
            permissions: actions(&r.permissions, |p| match p {
                v2::TopicApiPermissions::TopicList => List,
                v2::TopicApiPermissions::TopicRead => Read,
                v2::TopicApiPermissions::TopicWrite => Write,
            })?,
            topics: match required(r.topic, "topic selector")? {
                topic_rule::Topic::AllTopics(_) => PrefixSelector::All,
                topic_rule::Topic::TopicSelector(s) => match required(s.kind, "topic selector")? {
                    v2::topic_selector::Kind::TopicName(n) => PrefixSelector::Name(n),
                    v2::topic_selector::Kind::TopicNamePrefix(n) => PrefixSelector::Prefix(n),
                },
            },
            caches: match required(r.cache, "cache selector")? {
                topic_rule::Cache::AllCaches(_) => NameSelector::All,
                topic_rule::Cache::CacheSelector(s) => v2_cache(s)?,
            },
        },
        Kind::StorageRule(r) => Rule::Store {
            permissions: actions(&r.permissions, |p| match p {
                v2::StoreApiPermissions::StoreList => List,
                v2::StoreApiPermissions::StoreRead => Read,
                v2::StoreApiPermissions::StoreWrite => Write,
            })?,
            stores: match required(r.store, "store selector")? {
                store_rule::Store::AllStores(_) => NameSelector::All,
                store_rule::Store::StoreSelector(s) => match required(s.kind, "store selector")? {
                    v2::store_selector::Kind::StoreName(n) => NameSelector::Name(n),
                },
            },
            items: match r.store_item {
                None | Some(store_rule::StoreItem::AllItems(_)) => ItemSelector::All,
                Some(store_rule::StoreItem::ItemSelector(s)) => {
                    match required(s.kind, "item selector")? {
                        v2::store_item_selector::Kind::Key(k) => ItemSelector::Key(key(k)),
                        v2::store_item_selector::Kind::KeyPrefix(k) => {
                            ItemSelector::KeyPrefix(key(k))
                        }
                    }
                }
            },
        },
        Kind::FunctionRule(r) => Rule::Function {
            permissions: actions(&r.permissions, |p| match p {
                v2::FunctionApiPermissions::FunctionInvoke => Invoke,
            })?,
            functions: match required(r.function, "function selector")? {
                function_rule::Function::AllFunctions(_) => PrefixSelector::All,
                function_rule::Function::FunctionSelector(s) => {
                    match required(s.kind, "function selector")? {
                        v2::function_selector::Kind::FunctionName(n) => PrefixSelector::Name(n),
                        v2::function_selector::Kind::FunctionPrefix(n) => PrefixSelector::Prefix(n),
                    }
                }
            },
            caches: match required(r.cache, "cache selector")? {
                function_rule::Cache::AllCaches(_) => NameSelector::All,
                function_rule::Cache::CacheSelector(s) => v2_cache(s)?,
            },
        },
        Kind::DatabaseRule(r) => Rule::Database {
            permissions: match required(r.permissions, "database permissions")? {
                database_rule::Permissions::ApiPermissions(list) => {
                    actions(&list.permissions, |p| match p {
                        v2::DatabaseApiPermissions::DatabaseRead => Read,
                        v2::DatabaseApiPermissions::DatabaseWrite => Write,
                    })?
                }
            },
            databases: match required(r.database, "database selector")? {
                database_rule::Database::AllDatabases(_) => NameSelector::All,
                database_rule::Database::DatabaseSelector(s) => {
                    match required(s.kind, "database selector")? {
                        v2::database_selector::Kind::DatabaseName(n) => NameSelector::Name(n),
                    }
                }
            },
            items: match r.database_item {
                None | Some(database_rule::DatabaseItem::AllItems(_)) => ItemSelector::All,
                Some(database_rule::DatabaseItem::ItemSelector(s)) => {
                    match required(s.kind, "item selector")? {
                        v2::database_item_selector::Kind::Key(k) => ItemSelector::Key(key(k)),
                        v2::database_item_selector::Kind::KeyPrefix(k) => {
                            ItemSelector::KeyPrefix(key(k))
                        }
                    }
                }
            },
        },
    })
}

fn v2_cache(selector: v2::CacheSelector) -> Result<NameSelector, CliError> {
    match required(selector.kind, "cache selector")? {
        v2::cache_selector::Kind::CacheName(name) => Ok(NameSelector::Name(name)),
    }
}

fn v2_condition(condition: v2::Condition) -> Result<Condition, CliError> {
    match required(condition.condition, "condition")? {
        v2::condition::Condition::IpFilter(filter) => Ok(Condition::IpFilter {
            allowed_cidr_ranges: filter
                .allowed_cidr_ranges
                .into_iter()
                .map(|range| match required(range.range, "CIDR range")? {
                    v2::ip_filter::cidr_range::Range::Ipv4(cidr)
                    | v2::ip_filter::cidr_range::Range::Ipv6(cidr) => Ok(cidr),
                })
                .collect::<Result<_, CliError>>()?,
        }),
    }
}

impl Permissions {
    /// v1 permissions: the `p` claim on v1 API tokens and v1 disposable tokens.
    /// An empty claim is a super-user key and never reaches here.
    pub fn from_v1(permissions: PermissionsProtoV1) -> Result<Self, CliError> {
        match required(permissions.kind, "permissions")? {
            v1::permissions::Kind::SuperUser(_) => Ok(super_user()),
            v1::permissions::Kind::Explicit(explicit) => Ok(Permissions {
                super_user: None,
                rules: Some(
                    explicit
                        .permissions
                        .into_iter()
                        .map(v1_rule)
                        .collect::<Result<_, _>>()?,
                ),
                conditions: None,
            }),
        }
    }

    /// v2 permissions: the `p` claim on v2 disposable tokens.
    pub fn from_v2(permissions: PermissionsProtoV2) -> Result<Self, CliError> {
        match required(permissions.kind, "permissions")? {
            v2::permission_set::Kind::SuperUser(_) => Ok(super_user()),
            v2::permission_set::Kind::Explicit(explicit) => Ok(Permissions {
                super_user: None,
                rules: Some(
                    explicit
                        .rules
                        .into_iter()
                        .map(v2_rule)
                        .collect::<Result<_, _>>()?,
                ),
                conditions: Some(
                    permissions
                        .conditions
                        .into_iter()
                        .map(v2_condition)
                        .collect::<Result<_, _>>()?,
                ),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v1_roles_become_actions_and_an_unset_item_selector_is_every_item() {
        let permissions = PermissionsProtoV1 {
            kind: Some(v1::permissions::Kind::Explicit(v1::ExplicitPermissions {
                permissions: vec![v1::PermissionsType {
                    kind: Some(v1::permissions_type::Kind::CachePermissions(
                        v1::permissions_type::CachePermissions {
                            role: v1::CacheRole::CacheReadOnly as i32,
                            cache: Some(
                                v1::permissions_type::cache_permissions::Cache::CacheSelector(
                                    v1::permissions_type::CacheSelector {
                                        kind: Some(
                                            v1::permissions_type::cache_selector::Kind::CacheName(
                                                "my-cache".to_owned(),
                                            ),
                                        ),
                                    },
                                ),
                            ),
                            cache_item: None,
                        },
                    )),
                }],
            })),
        };
        let rules = Permissions::from_v1(permissions)
            .expect("v1 permissions convert")
            .rules
            .expect("explicit permissions have rules");
        assert_eq!(
            vec![Rule::Cache {
                permissions: vec![PermissionAction::Read],
                caches: NameSelector::Name("my-cache".to_owned()),
                items: ItemSelector::All,
            }],
            rules,
        );
    }

    #[test]
    fn v2_explicit_permissions_keep_their_conditions() {
        let permissions = PermissionsProtoV2 {
            kind: Some(v2::permission_set::Kind::Explicit(
                v2::ExplicitPermissions { rules: vec![] },
            )),
            conditions: vec![v2::Condition {
                condition: Some(v2::condition::Condition::IpFilter(v2::IpFilter {
                    allowed_cidr_ranges: vec![v2::ip_filter::CidrRange {
                        range: Some(v2::ip_filter::cidr_range::Range::Ipv4(
                            "10.0.0.0/8".to_owned(),
                        )),
                    }],
                })),
            }],
        };
        let converted = Permissions::from_v2(permissions).expect("v2 permissions convert");
        assert_eq!(
            Some(vec![Condition::IpFilter {
                allowed_cidr_ranges: vec!["10.0.0.0/8".to_owned()],
            }]),
            converted.conditions,
        );
    }

    #[test]
    fn an_unknown_permission_value_is_an_error_not_a_silent_drop() {
        let rule = v2::Rule {
            kind: Some(v2::rule::Kind::CacheRule(v2::rule::CacheRule {
                permissions: vec![99],
                cache: Some(v2::rule::cache_rule::Cache::AllCaches(v2::All {})),
                cache_item: None,
            })),
        };
        assert!(v2_rule(rule).is_err());
    }
}
