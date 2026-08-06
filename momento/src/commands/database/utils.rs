use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct DatabaseResponse {
    pub name: String,
    pub pool_name: String,
}

#[derive(Deserialize)]
pub struct DatabaseError {
    pub detail: Option<String>,
    pub message: Option<String>,
}
