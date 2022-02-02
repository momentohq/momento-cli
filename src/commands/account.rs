use std::collections::HashMap;

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
}

impl Default for CreateTokenResponse {
    fn default() -> Self {
        Self {
            message: "Unable to create token".to_string(),
        }
    }
}

pub async fn signup_user(email: String, region: String) -> Result<(), CliError> {
    // This is a temporarily solution until we have figured out how we want to handle
    // auth across multiple cells. This solution will not work once we have more than one
    // cell per region. Our cellular design supports this, and we most definitely will
    // run into this issue in the future.
    let region_to_cell_name_map: HashMap<&str, &str> = [
        ("us-west-2", "cell-external-beta"),
        ("us-east-1", "cell-us-east-1"),
    ]
    .iter()
    .cloned()
    .collect();
    if !region_to_cell_name_map.contains_key(region.as_str()) {
        return Err(CliError {
            msg: format!(
                "Unsupported region passed. Supported regions are {:#?}",
                region_to_cell_name_map.keys()
            ),
        });
    }
    // All of our production envs are hardcoded to 1 as of right now. This is something
    // that we also might have to revisit if it ever changes.
    let env = "1";
    let cell_name = region_to_cell_name_map.get(region.as_str()).unwrap();
    let url = format!(
        "https://identity.{}-{}.prod.a.momentohq.com/token/create",
        cell_name, env
    );

    let body = &CreateTokenBody { email };
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
