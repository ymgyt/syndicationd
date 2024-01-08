use crate::{
    application::{AuthenticateMethod, Direction},
    auth::device_flow::{DeviceAccessTokenResponse, DeviceAuthorizationResponse},
    client::query::user::UserSubscription,
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
    FetchSubscription,
    UpdateSubscription(UserSubscription),
}
