use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;

use crate::{error::CliError, utils::console::console_info};

const SIGNUP_ENDPOINT: &str = "https://signup.registry.prod.a.momentohq.com";

#[derive(Deserialize, Debug)]
struct CreateTokenResponse {
    message: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct CreateTokenBody {
    email: String,
    cloud: String,
    region: String,
}

impl Default for CreateTokenResponse {
    fn default() -> Self {
        Self {
            message: "Unable to create token".to_string(),
        }
    }
}

fn get_signup_endpoint() -> String {
    env::var("MOMENTO_SIGNUP_ENDPOINT").unwrap_or_else(|_| String::from(SIGNUP_ENDPOINT))
}

pub async fn signup_user(email: String, cloud: String, region: String) -> Result<(), CliError> {
    let endpoint = get_signup_endpoint();
    let url = format!("{}/token/create", endpoint);

    let body = &CreateTokenBody {
        email,
        cloud,
        region,
    };
    console_info!("Signing up for Momento...");
    match Client::new().post(url).json(body).send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                console_info!("Success! Your access token will be emailed to you shortly.")
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
