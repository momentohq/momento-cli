use momento::{preview::topics::Subscription, MomentoResult};
use futures::StreamExt;

use crate::utils::console::console_data;

pub async fn print_subscription(mut subscription: Subscription) -> MomentoResult<()> {
    while let Some(item) = subscription.next().await {
        match item {
            momento::preview::topics::SubscriptionItem::Value(value) => match value.kind {
                momento::preview::topics::ValueKind::Text(text) => console_data!("{text}"),
                momento::preview::topics::ValueKind::Binary(binary) => {
                    console_data!("{{\"kind\": \"binary\", \"length\": {}}}", binary.len())
                }
            },
            momento::preview::topics::SubscriptionItem::Discontinuity(discontinuity) => {
                console_data!("{discontinuity:?}")
            }
        }
    }
    Ok(())
}
