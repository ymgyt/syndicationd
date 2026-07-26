use synd_client::payload;
use tracing::debug;

use crate::{
    application::{Populate, input_parser::InputParser},
    event::{FeedRequestEvent, GhEvent},
    operation::{Operation, Operations},
};

use super::{Components, FeedsComponent};

impl Components {
    pub(in crate::application) fn apply_feed_subscription_editor_closed(
        &mut self,
        input: &str,
    ) -> Option<Operation> {
        match InputParser::new(input).parse_feed_subscription(&self.shell.categories) {
            Ok(input) if self.feeds.is_already_subscribed(&input.url) => {
                self.shell
                    .prompt
                    .set_error_message(format!("{} already subscribed", input.url));
                None
            }
            Ok(input) => Some(Operation::SubscribeFeed { input }),
            Err(error) => {
                self.shell.prompt.set_error_message(error.to_string());
                None
            }
        }
    }

    pub(in crate::application) fn apply_feed_edition_editor_closed(
        &mut self,
        input: &str,
    ) -> Option<Operation> {
        match InputParser::new(input).parse_feed_subscription(&self.shell.categories) {
            Ok(input) => Some(Operation::SubscribeFeed { input }),
            Err(error) => {
                self.shell.prompt.set_error_message(error.to_string());
                None
            }
        }
    }

    pub(in crate::application) fn apply_feed_push(
        &mut self,
        event: payload::FeedEvent,
    ) -> Option<Operation> {
        match event {
            payload::FeedEvent::TimelineChanged(event) => {
                debug!(
                    changed_at = %event.changed_at,
                    affected_feeds = ?event.affected_feeds,
                    "timeline changed"
                );
                self.feeds.refresh_timeline()
            }
        }
    }

    pub(in crate::application) fn apply_feed_request_event(
        &mut self,
        event: FeedRequestEvent,
        feeds_first: i64,
        entries_limit: usize,
    ) -> Option<Operation> {
        match event {
            FeedRequestEvent::FeedSubscribed { url } => {
                debug!(%url, "feed subscribed");
                Some(FeedsComponent::reload_subscription(feeds_first))
            }
            FeedRequestEvent::FeedUnsubscribed { url } => {
                self.feeds.feed_unsubscribed(&url);
                self.refresh_feed_categories();
                None
            }
            FeedRequestEvent::SubscriptionFetched {
                populate,
                subscription,
            } => self.apply_subscription_fetched(populate, subscription),
            FeedRequestEvent::TimelineWindowChunkFetched { entries, base_seq } => {
                self.feeds
                    .apply_timeline_window_chunk(entries, base_seq, entries_limit);
                self.refresh_feed_categories();
                None
            }
            FeedRequestEvent::TimelineChangesFetched { changes, seq } => {
                self.feeds
                    .apply_timeline_changes(changes, seq, entries_limit);
                self.refresh_feed_categories();
                None
            }
        }
    }

    fn apply_subscription_fetched(
        &mut self,
        populate: Populate,
        subscription: payload::SubscriptionPayload,
    ) -> Option<Operation> {
        let next_page = match &subscription.feeds.page_info {
            payload::PageInfo::Complete { .. } => None,
            payload::PageInfo::More { next_cursor } => Some(Operation::FetchSubscription {
                populate: Populate::Append,
                after: Some(next_cursor.clone()),
                first: subscription.feeds.nodes.len().try_into().unwrap_or(0),
            }),
        };
        self.feeds
            .subscription
            .update_subscription(populate, subscription);
        next_page
    }

    fn refresh_feed_categories(&mut self) {
        self.shell.filter.update_categories(
            &self.shell.categories,
            Populate::Replace,
            self.feeds.entries.entries(),
        );
    }

    pub(in crate::application) fn apply_gh_event(&mut self, event: GhEvent) -> Operations {
        match event {
            GhEvent::NotificationsFetched {
                notifications,
                populate,
            } => {
                let details = self.gh.apply_notifications(populate, notifications);
                if populate == Populate::Replace {
                    self.shell.filter.clear_gh_notifications_categories();
                }
                self.gh.fetch_notification_details(details).into()
            }
            GhEvent::IssueFetched {
                notification_id,
                issue,
            } => {
                if let Some(notification) = self.gh.notifications.update_issue(
                    notification_id,
                    issue,
                    &self.shell.categories,
                ) {
                    self.shell.filter.update_gh_notification_categories(
                        &self.shell.categories,
                        Populate::Append,
                        notification.categories().cloned(),
                    );
                }
                Operations::Nop
            }
            GhEvent::PullRequestFetched {
                notification_id,
                pull_request,
            } => {
                if let Some(notification) = self.gh.notifications.update_pull_request(
                    notification_id,
                    pull_request,
                    &self.shell.categories,
                ) {
                    self.shell.filter.update_gh_notification_categories(
                        &self.shell.categories,
                        Populate::Append,
                        notification.categories().cloned(),
                    );
                }
                Operations::Nop
            }
            GhEvent::NotificationMarkedAsDone { notification_id } => {
                self.gh.notifications.marked_as_done(notification_id);
                Operations::Nop
            }
        }
    }
}
