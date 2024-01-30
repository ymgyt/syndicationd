use std::fmt::Display;

use crate::{
    application::Direction,
    auth::{
        device_flow::{DeviceAccessTokenResponse, DeviceAuthorizationResponse},
        AuthenticationProvider,
    },
    client::{payload, query::subscription::SubscriptionOutput},
    types::Feed,
};

#[derive(Debug)]
pub enum Command {
    Quit,
    ResizeTerminal { columns: u16, rows: u16 },
    Idle,

    Authenticate(AuthenticationProvider),
    DeviceAuthorizationFlow(DeviceAuthorizationResponse),
    CompleteDevieAuthorizationFlow(DeviceAccessTokenResponse),

    MoveTabSelection(Direction),

    // Subscription
    MoveSubscribedFeed(Direction),
    PromptFeedSubscription,
    PromptFeedUnsubscription,
    SubscribeFeed { url: String },
    UnsubscribeFeed { url: String },
    CompleteSubscribeFeed { feed: Feed },
    CompleteUnsubscribeFeed { url: String },
    FetchSubscription { after: Option<String>, first: i64 },
    UpdateSubscription(SubscriptionOutput),
    OpenFeed,

    // Entries
    FetchEntries { after: Option<String>, first: i64 },
    UpdateEntries(payload::FetchEntriesPayload),
    MoveEntry(Direction),
    OpenEntry,

    HandleError { message: String },
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::UpdateSubscription(_) => f.write_str("UpdateSubscription"),
            Command::UpdateEntries(_) => f.write_str("UpdateEntries"),
            cmd => write!(f, "{:?}", cmd),
        }
    }
}
