use log::info;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
struct CreateTokenResponse {
    message: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct CreateTokenBody {
    email: String,
}

impl Default for CreateTokenResponse {
    fn default() -> Self {
        Self {
            message: "Unable to create token".to_string(),
        }
    }
}

pub async fn signup_user(email: String) {
    let url = "https://identity.cell-external-beta-1.prod.a.momentohq.com/token/create";

    let body = &CreateTokenBody { email };
    info!("Signing up for Momento...");
    match Client::new().post(url).json(body).send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                info!("Success! Your access token will be emailed to you shortly.")
            } else {
                let response_json: CreateTokenResponse = resp.json().await.unwrap_or_default();
                panic!("Failed to create Momento token: {}", response_json.message)
            }
        }
        Err(e) => panic!("{}", e),
    };
}
