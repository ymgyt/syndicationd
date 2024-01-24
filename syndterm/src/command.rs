use crate::{
    application::{AuthenticateMethod, Direction},
    auth::device_flow::{DeviceAccessTokenResponse, DeviceAuthorizationResponse},
    client::query::subscription::SubscriptionOutput,
    types::FeedMeta,
};

#[derive(Debug)]
pub enum Command {
    Quit,
    ResizeTerminal { columns: u16, rows: u16 },
    Authenticate(AuthenticateMethod),
    DeviceAuthorizationFlow(DeviceAuthorizationResponse),
    CompleteDevieAuthorizationFlow(DeviceAccessTokenResponse),
    MoveTabSelection(Direction),
    MoveSubscribedFeed(Direction),
    PromptFeedSubscription,
    PromptFeedUnsubscription,
    SubscribeFeed { url: String },
    UnsubscribeFeed { url: String },
    CompleteSubscribeFeed { feed: FeedMeta },
    CompleteUnsubscribeFeed { url: String },
    FetchSubscription { after: Option<String>, first: i64 },
    UpdateSubscription(SubscriptionOutput),
    OpenFeed,
    HandleError { message: String },
}
