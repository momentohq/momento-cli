use serde::{Deserialize, Serialize};

pub const ENV_VAR_NAME_MOMENTO_CONFIG_DIR: &str = "MOMENTO_CONFIG_DIR";
pub const DEFAULT_CACHE_NAME: &str = "default-cache";

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct Config {
    pub cache: String,
    pub ttl: u64,
}

#[derive(Deserialize, Serialize, Clone, Default, Debug)]
pub struct Credentials {
    pub token: String,
}
