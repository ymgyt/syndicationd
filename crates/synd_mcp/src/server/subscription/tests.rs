use synd_client::{
    FeedUrl,
    payload::{
        ResponseCode, ResponseStatus, SubscribeDisposition, SubscribeFeedPayload,
        UnsubscribeDisposition, UnsubscribeFeedPayload,
    },
};

use super::SubscriptionChange;
use crate::error::ToolError;

#[test]
fn preserves_an_unrecognized_subscribe_disposition() {
    let response = SubscribeFeedPayload {
        status: ResponseStatus {
            code: ResponseCode::Ok,
        },
        url: FeedUrl::parse("https://example.com/feed").unwrap(),
        disposition: SubscribeDisposition::Other("UNCHANGED".into()),
    };
    let change = SubscriptionChange::try_from(response).unwrap();
    assert_eq!(change.disposition, "UNCHANGED");
}

#[test]
fn a_failed_unsubscribe_status_takes_precedence_over_the_disposition() {
    let response = UnsubscribeFeedPayload {
        status: ResponseStatus {
            code: ResponseCode::Unauthorized,
        },
        url: FeedUrl::parse("https://example.com/feed").unwrap(),
        disposition: UnsubscribeDisposition::Unsubscribed,
    };
    assert!(matches!(
        SubscriptionChange::try_from(response),
        Err(ToolError::SubscriptionFailed(ResponseCode::Unauthorized))
    ));
}
