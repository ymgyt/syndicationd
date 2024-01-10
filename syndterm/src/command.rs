use crate::{
    application::{AuthenticateMethod, Direction},
    auth::device_flow::{DeviceAccessTokenResponse, DeviceAuthorizationResponse},
    client::query::subscription::SubscriptionOutput,
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
    CompleteSubscribeFeed { url: String },
    FetchSubscription,
    UpdateSubscription(SubscriptionOutput),
    HandleError { message: String },
}
