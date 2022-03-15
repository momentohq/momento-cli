use log::info;
use reqwest::header::{HeaderMap, ACCEPT, USER_AGENT};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::error::CliError;

#[derive(Serialize, Deserialize, Debug)]
struct LatestReleaseResponse {
    name: String,
}

impl Default for LatestReleaseResponse {
    fn default() -> Self {
        Self {
            name: "Unable to retrieve the latest release version".to_string(),
        }
    }
}

pub async fn get_version() -> Result<(), CliError> {
    let url = "https://api.github.com/repos/momentohq/momento-cli/releases/latest";
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, "request".parse().unwrap());
    headers.insert(ACCEPT, "application/vnd.github.v3+json".parse().unwrap());
    match Client::new().post(url).headers(headers).send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                let response_json: LatestReleaseResponse = resp.json().await.unwrap_or_default();
                info!("Momento Version: {}", response_json.name);
            } else {
                return Err(CliError {
                    msg: format!("Failed at {}", resp.status()),
                });
            }
        }
        Err(e) => {
            return Err(CliError {
                msg: format!("Here: {}", e.to_string()),
            })
        }
    };

    Ok(())
}
