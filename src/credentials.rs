use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
pub struct CredentialsConfig {
    pub profile: HashMap<String, Credentials>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Credentials {
    pub token: String,
}
