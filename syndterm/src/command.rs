use crate::{
    application::{AuthenticateMethod, Direction},
    auth::device_flow::{DeviceAccessTokenResponse, DeviceAuthorizationResponse},
    client::query::subscription::SubscriptionOutput,
    types::FeedMeta,
};

#[derive(Debug)]
pub enum Command {
    Quit,
    Authenticate(AuthenticateMethod),
    DeviceAuthorizationFlow(DeviceAuthorizationResponse),
    CompleteDevieAuthorizationFlow(DeviceAccessTokenResponse),
    MoveTabSelection(Direction),
    PromptFeedSubscription,
    SubscribeFeed { url: String },
    CompleteSubscribeFeed { feed: FeedMeta },
    FetchSubscription { after: Option<String>, first: i64 },
    UpdateSubscription(SubscriptionOutput),
    HandleError { message: String },
}
