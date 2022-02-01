use std::collections::HashMap;

use log::info;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct CreateTokenBody {
    email: String,
}

#[derive(Deserialize, Debug)]
struct CreateTokenResponse {
    message: String,
}

impl Default for CreateTokenResponse {
    fn default() -> Self {
        Self {
            message: "Unable to create token".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct CreatePayload {
    text: String,
    channel: String,
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
    let hook = String::from(
        "https://hooks.slack.com/services/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",
    );
    let channel = "C031S3VLJF2";

    let body = &CreateTokenBody { email };
    let token_payload = &CreatePayload {
        text: String::from(format!(
            "<!here> \n{} failed to create Momento token in {} :meow-sad-life:",
            body.email, region
        )),
        channel: channel.to_string(),
    };
    let sign_up_payload = &CreatePayload {
        text: String::from(format!(
            "<!here> \nSomething went wrong during sign up for {} in {} :meow-sad-life:",
            body.email, region
        )),
        channel: channel.to_string(),
    };
    info!("Signing up for Momento...");
    match Client::new().post(url).json(body).send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                info!("Success! Your access token will be emailed to you shortly.");
            } else {
                match Client::new().post(hook).json(token_payload).send().await {
                    Ok(_resp) => {}
                    Err(_e) => {
                        // Displays this error message when both the lambda and sending Slack notification request fail
                        panic!("Sorry, we were unable to create a token for you, please contact support@momentohq.com to get your token")
                    }
                }
                let response_json: CreateTokenResponse = resp.json().await.unwrap_or_default();
                // Displays this error message only when the lambda fails
                panic!(
                    "Sorry, we were unable to create Momento token: {}",
                    response_json.message
                )
            }
        }
        Err(_e) => {
            match Client::new().post(hook).json(sign_up_payload).send().await {
                Ok(_resp) => {}
                Err(_e) => {
                    // Displays this error message when sign up request and sending Slack notification request fail
                    panic!("Sorry, we were unable to sign you up, please contact support@momentohq.com to complete your signup")
                }
            }
            // Displays this error message only when sign up request fails
            panic!("Sorry, we were unable to sign you up, please contact support@momentohq.com to complete your signup")
        }
    };
}
