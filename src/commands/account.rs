use std::collections::HashMap;

use log::info;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct CreateTokenBody {
    email: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct CreatePayload {
    text: String,
    channel: String,
}

async fn send_slack_notification(
    payload: &CreatePayload,
    success_message: Option<String>,
    error_message: Option<String>,
) {
    let hook = String::from(
        "https://hooks.slack.com/services/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",
    );
    if success_message.is_none() && error_message.is_none() {
        match Client::new().post(hook).json(payload).send().await {
            Ok(_resp) => {}
            Err(_e) => {}
        }
    } else {
        if success_message.is_some() && error_message.is_some() {
            match Client::new().post(hook).json(payload).send().await {
                Ok(_resp) => {
                    info!("{:?}", success_message)
                }
                Err(_e) => {
                    panic!("{:?}", error_message)
                }
            }
        } else {
            if success_message.is_some() {
                match Client::new().post(hook).json(payload).send().await {
                    Ok(_resp) => {
                        info!("{:?}", success_message)
                    }
                    Err(_e) => {}
                }
            } else {
                match Client::new().post(hook).json(payload).send().await {
                    Ok(_resp) => {}
                    Err(_e) => {
                        panic!("{:?}", error_message)
                    }
                }
            }
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
    // Channel ID for cli-signups channel
    let channel = "C031S3VLJF2";
    let body = &CreateTokenBody { email };

    info!("Signing up for Momento...");
    match Client::new().post(url).json(body).send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                info!("Success! Your access token will be emailed to you shortly.");
                let token_payload = &CreatePayload {
                    text: String::from(format!(
                        "*Success* :white_check_mark: \nToken successfully sent to {} in {}",
                        body.email, region
                    )),
                    channel: channel.to_string(),
                };
                send_slack_notification(token_payload, None, None).await;
            } else {
                let token_payload =
                    &CreatePayload {
                        text: String::from(format!(
                        "<!here> \n*Failed* :x: \nFailed to send token to {} in {} \nStatus: {:?}",
                        body.email, region, resp.status()
                    )),
                        channel: channel.to_string(),
                    };
                let success_message: Option<String> =
                    Some("Success! Your access token will be emailed to you shortly.".to_owned());
                let error_message: Option<String> = Some("Sorry, we were unable to create a token for you, please try later or contact support@momentohq.com to get your token".to_owned());
                send_slack_notification(token_payload, success_message, error_message).await;
            }
        }
        Err(e) => {
            let token_payload =
                    &CreatePayload {
                        text: String::from(format!(
                        "<!here> \n*Failed* :x: \nSign up lambda request failed \nError: {} \nEmail: {} \nRegion: {}",
                        e.to_string(), body.email, region
                    )),
                        channel: channel.to_string(),
                    };
            let success_message: Option<String> =
                Some("Success! Your access token will be emailed to you shortly.".to_owned());
            let error_message: Option<String> = Some("Sorry, we were unable to create a token for you, please try later or contact support@momentohq.com to get your token".to_owned());
            send_slack_notification(token_payload, success_message, error_message).await;
        }
    };
}
