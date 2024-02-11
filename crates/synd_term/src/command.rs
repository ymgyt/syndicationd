use std::fmt::Display;
use synd_authn::device_flow::{DeviceAccessTokenResponse, DeviceAuthorizationResponse};

use crate::{
    application::{Direction, ListAction, RequestSequence},
    auth::AuthenticationProvider,
    client::{payload, query::subscription::SubscriptionOutput},
    types::Feed,
};

#[derive(Debug)]
pub enum Command {
    Quit,
    ResizeTerminal {
        columns: u16,
        rows: u16,
    },
    RenderThrobber,
    Idle,

    Authenticate(AuthenticationProvider),
    DeviceAuthorizationFlow(DeviceAuthorizationResponse),
    CompleteDevieAuthorizationFlow(DeviceAccessTokenResponse),

    MoveTabSelection(Direction),

    // Subscription
    MoveSubscribedFeed(Direction),
    PromptFeedSubscription,
    PromptFeedUnsubscription,
    SubscribeFeed {
        url: String,
    },
    UnsubscribeFeed {
        url: String,
    },
    CompleteSubscribeFeed {
        feed: Feed,
        request_seq: RequestSequence,
    },
    CompleteUnsubscribeFeed {
        url: String,
        request_seq: RequestSequence,
    },
    FetchSubscription {
        after: Option<String>,
        first: i64,
    },
    UpdateSubscription {
        subscription: SubscriptionOutput,
        request_seq: RequestSequence,
    },
    OpenFeed,

    // Entries
    FetchEntries {
        after: Option<String>,
        first: i64,
    },
    UpdateEntries {
        action: ListAction,
        payload: payload::FetchEntriesPayload,
        request_seq: RequestSequence,
    },
    MoveEntry(Direction),
    OpenEntry,

    HandleError {
        message: String,
        request_seq: Option<RequestSequence>,
    },
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::UpdateSubscription { .. } => f.write_str("UpdateSubscription"),
            Command::UpdateEntries { .. } => f.write_str("UpdateEntries"),
            cmd => write!(f, "{cmd:?}"),
        }
    }
}
