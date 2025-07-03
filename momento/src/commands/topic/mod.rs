use futures::StreamExt;
use momento::{topics::Subscription, MomentoResult};

use crate::utils::console::console_data;

pub async fn print_subscription(mut subscription: Subscription) -> MomentoResult<()> {
    while let Some(item) = subscription.next().await {
        match item.kind {
            momento::topics::ValueKind::Text(text) => console_data!("{:?}", text),
            momento::topics::ValueKind::Binary(binary) => {
                console_data!("{:?}", binary)
            }
        }
    }
    Ok(())
}
