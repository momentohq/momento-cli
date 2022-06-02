use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub enum FileTypes {
    Config(Config),
    Credentials(Credentials),
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct Config {
    pub cache: String,
    pub ttl: u64,
}

#[derive(Deserialize, Serialize, Clone, Default, Debug)]
pub struct Credentials {
    pub token: String,
}
