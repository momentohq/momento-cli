use core::panic;

use chrono::Utc;
use serde::{Deserialize, Serialize};

pub const ENV_VAR_NAME_MOMENTO_CONFIG_DIR: &str = "MOMENTO_CONFIG_DIR";
pub const DEFAULT_CACHE_NAME: &str = "default-cache";

pub const CONFIG_CACHE_KEY: &str = "cache";
pub const CONFIG_TTL_KEY: &str = "ttl";

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct Config {
    pub cache: String,
    pub ttl: u64,
}

pub const CREDENTIALS_TOKEN_KEY: &str = "token";
pub const CREDENTIALS_VALID_FOR_KEY: &str = "valid_for";

#[derive(Deserialize, Serialize, Clone, Default, Debug)]
pub struct Credentials {
    pub token: String,
    pub valid_for: Option<i64>,
}

impl Credentials {
    pub fn new<S>(token: S, valid_for: Option<i64>) -> Self
    where
        S: Into<String>,
    {
        Self {
            token: token.into(),
            valid_for,
        }
    }

    pub fn new_from_duration<S>(token: S, valid_for: std::time::Duration) -> Self
    where
        S: Into<String>,
    {
        let valid_for = match chrono::Duration::from_std(valid_for) {
            Ok(inner) => inner,
            Err(_) => panic!("Hit out of range error, this shouldn't have happend"),
        };
        let valid_for = (Utc::now() + valid_for).timestamp();
        Self {
            token: token.into(),
            valid_for: Some(valid_for),
        }
    }

    pub fn valid_forever<S>(token: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            token: token.into(),
            valid_for: None,
        }
    }
}
