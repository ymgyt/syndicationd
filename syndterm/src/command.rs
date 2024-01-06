use crate::{
    application::AuthenticateMethod,
    auth::device_flow::{DeviceAccessTokenResponse, DeviceAuthorizationResponse},
    client::query::user::UserSubscription,
};

#[derive(Debug)]
pub enum Command {
    Quit,
    Authenticate(AuthenticateMethod),
    DeviceAuthorizationFlow(DeviceAuthorizationResponse),
    CompleteDevieAuthorizationFlow(DeviceAccessTokenResponse),
    FetchSubscription,
    UpdateSubscription(UserSubscription),
}
