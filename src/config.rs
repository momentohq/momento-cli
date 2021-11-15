use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct Profiles<T>
where
    T: Default,
{
    pub profile: HashMap<String, T>,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct Config {
    pub cache: String,
    pub ttl: u32,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct Credentials {
    pub token: String,
}
