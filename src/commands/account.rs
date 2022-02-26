use log::info;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::error::CliError;

const SIGNUP_ENDPOINT: &str = "https://identity.prod.a.momentohq.com";

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

pub async fn signup_user(email: String, region: String, signup_url: Option<String>) -> Result<(), CliError> {
    let url = format!("{}/token/create", signup_url.unwrap_or(String::from(SIGNUP_ENDPOINT)));

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
