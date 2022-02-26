use log::info;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::error::CliError;

#[derive(Deserialize, Debug)]
struct CreateTokenResponse {
    message: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct CreateTokenBody {
    email: String,
    region: String,
}

impl Default for CreateTokenResponse {
    fn default() -> Self {
        Self {
            message: "Unable to create token".to_string(),
        }
    }
}

pub async fn signup_user(email: String, region: String) -> Result<(), CliError> {
    let url = "https://identity.prod.a.momentohq.com/token/create";

    let body = &CreateTokenBody { email , region};
    info!("Signing up for Momento...");
    match Client::new().post(url).json(body).send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                info!("Success! Your access token will be emailed to you shortly.")
            } else {
                let response_json: CreateTokenResponse = resp.json().await.unwrap_or_default();
                return Err(CliError {
                    msg: format!("Failed to create Momento token: {}", response_json.message),
                });
            }
        }
        Err(e) => return Err(CliError { msg: e.to_string() }),
    };

    Ok(())
}
