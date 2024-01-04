use crate::client::query::user::UserSubscription;

pub enum Command {
    Quit,
    FetchSubscription,
    UpdateSubscription(UserSubscription),
}
