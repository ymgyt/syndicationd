use crate::subscription::{SubscriberId, Subscription};

#[derive(Debug, Clone)]
pub struct SubscriptionsQuery {
    pub subscriber_id: SubscriberId,
    pub after: Option<String>,
    pub first: usize,
}

#[derive(Debug, Clone)]
pub struct Subscriptions {
    pub subscriptions: Vec<Subscription>,
    pub has_next_page: bool,
    pub end_cursor: Option<String>,
}

impl Subscriptions {
    pub fn from_subscriptions(
        subscriptions: Vec<Subscription>,
        has_next_page: bool,
        end_cursor: Option<String>,
    ) -> Self {
        Self {
            subscriptions,
            has_next_page,
            end_cursor,
        }
    }
}
