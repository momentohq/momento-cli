use std::collections::HashMap;

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

#[derive(Deserialize, Debug)]
struct CreateSlackResponse {
    message: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct CreatePayload {
    text: String,
    channel: String
}

impl Default for CreateSlackResponse {
    fn default() -> Self {
        Self {
            message: "Unable to send Slack request".to_string(),
        }
    }
}


pub async fn signup_user(email: String, region: String) {
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
        panic!(
            "Unsupported region passed. Supported regions are {:#?}",
            region_to_cell_name_map.keys()
        )
    }
    // All of our production envs are hardcoded to 1 as of right now. This is something
    // that we also might have to revisit if it ever changes.
    let env = "1";
    let cell_name = region_to_cell_name_map.get(region.as_str()).unwrap();
    let url = format!(
        "https://identity.{}-{}.prod.a.momentohq.com/token/create",
        cell_name, env
    );
    let hook = String::from("https://hooks.slack.com/services/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");
    let channel = "C0311D2ARNF";

    let body = &CreateTokenBody { email };
    let token_payload =&CreatePayload {
        text: String::from(format!("<!here> \n{} failed to create Momento token in {} :meow-sad-life:", body.email, region)),
        channel: channel.to_string()
    };
    let sign_up_payload =&CreatePayload {
        text: String::from(format!("<!here> \nSomething went wrong during sign up for {} in {} :meow-sad-life:", body.email, region)),
        channel: channel.to_string()
    };
    info!("Signing up for Momento...");
    match Client::new().post(url).json(body).send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                info!("Success! Your access token will be emailed to you shortly.");
            } else {
                let response_json: CreateTokenResponse = resp.json().await.unwrap_or_default();
                match Client::new().post(hook).json(token_payload).send().await {
                    Ok(resp) => {
                        if resp.status().is_success() {
                        }
                        else {
                            let response_json: CreateSlackResponse = resp.json().await.unwrap_or_default();
                                panic!("Failed to send Slack request: {}", response_json.message)
                        }
                    }
                    Err(e) => panic!("{}", e),
                }
                panic!("Failed to create Momento token: {}", response_json.message)
            }
        }
        Err(e) => {
                match Client::new().post(hook).json(sign_up_payload).send().await {
                    Ok(resp) => {
                        if resp.status().is_success() {
                        }
                        else {
                            let response_json: CreateSlackResponse = resp.json().await.unwrap_or_default();
                                panic!("Failed to send Slack request: {}", response_json.message)
                        }
                    }
                    Err(e) => panic!("{}", e),
                }
            panic!("{}", e)
        },
    };
}
