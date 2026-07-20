use synd_client::payload;
use tracing::{debug, warn};

use crate::{
    application::{Populate, RequestSequence, input_parser::InputParser},
    auth::AuthenticationProvider,
    event::{FeedsApiEvent, GitHubApiEvent},
    operation::Operation,
    ui::widgets::filter::Filterer,
};
use synd_auth::device_flow::DeviceAuthorizationResponse;
use url::Url;

use super::{AppComponent, FeedsComponent};

impl AppComponent {
    pub(in crate::application) fn apply_device_flow_authorization_received(
        &mut self,
        provider: AuthenticationProvider,
        device_authorization: DeviceAuthorizationResponse,
    ) -> Vec<Operation> {
        self.shell
            .auth
            .set_device_authorization_response(device_authorization.clone());

        let mut operations = Vec::new();
        if let Ok(url) = Url::parse(device_authorization.verification_uri().to_string().as_str()) {
            operations.push(Operation::OpenBrowser { url });
        }
        operations.push(Operation::PollDeviceFlowAccessToken {
            provider,
            device_authorization: Box::new(device_authorization),
        });
        operations
    }

    pub(in crate::application) fn apply_feed_subscription_editor_closed(
        &mut self,
        input: &str,
    ) -> Vec<Operation> {
        match InputParser::new(input).parse_feed_subscription(&self.shell.categories) {
            Ok(input) if self.feeds.is_already_subscribed(&input.url) => {
                let message = format!("{} already subscribed", input.url);
                self.shell.prompt.set_error_message(message);
                Vec::new()
            }
            Ok(input) => vec![Operation::SubscribeFeed { input }],
            Err(err) => {
                self.shell.prompt.set_error_message(err.to_string());
                Vec::new()
            }
        }
    }

    pub(in crate::application) fn apply_feed_edition_editor_closed(
        &mut self,
        input: &str,
    ) -> Vec<Operation> {
        match InputParser::new(input).parse_feed_subscription(&self.shell.categories) {
            Ok(input) => vec![Operation::SubscribeFeed { input }],
            Err(err) => {
                self.shell.prompt.set_error_message(err.to_string());
                Vec::new()
            }
        }
    }

    pub(in crate::application) fn apply_timeline_changed(
        &mut self,
        event: &payload::TimelineChangeEvent,
    ) -> Vec<Operation> {
        debug!(
            changed_at = event.changed_at,
            affected_feeds = ?event.affected_feeds,
            "timeline changed"
        );
        self.mark_timeline_dirty()
    }

    pub(in crate::application) fn apply_feed_event(
        &mut self,
        event: payload::FeedEvent,
    ) -> Vec<Operation> {
        match event {
            payload::FeedEvent::TimelineChanged(event) => self.apply_timeline_changed(&event),
        }
    }

    pub(in crate::application) fn mark_timeline_dirty(&mut self) -> Vec<Operation> {
        self.feeds.mark_timeline_dirty().into_iter().collect()
    }

    pub(in crate::application) fn apply_entry_fetch_started(
        &mut self,
        request_seq: RequestSequence,
        populate: Populate,
    ) {
        self.feeds.start_entry_fetch(request_seq, populate);
    }

    pub(in crate::application) fn apply_synd_api_error(&mut self, request_seq: RequestSequence) {
        self.feeds.forget_entry_fetch(request_seq);
    }

    #[must_use]
    pub(in crate::application) fn apply_filterer(
        &mut self,
        filterer: Filterer,
    ) -> Option<Operation> {
        match filterer {
            Filterer::Feed(filterer) => {
                self.feeds.entries.update_filterer(filterer.clone());
                self.feeds.subscription.update_filterer(filterer);
                None
            }
            Filterer::GhNotification(filterer) => {
                self.github.notifications.update_filterer(filterer);
                self.github
                    .notifications
                    .fetch_next_if_needed()
                    .map(|params| Operation::FetchGitHubNotifications {
                        populate: Populate::Append,
                        params,
                    })
            }
        }
    }

    pub(in crate::application) fn apply_feeds_api_event(
        &mut self,
        request_seq: RequestSequence,
        event: FeedsApiEvent,
        feeds_first: i64,
        entries_first: i64,
        entries_limit: usize,
    ) -> Vec<Operation> {
        match event {
            // New entries arrive through the timeline change push once the
            // registry crawls the feed
            FeedsApiEvent::FeedSubscribed => {
                vec![FeedsComponent::reload_subscription(feeds_first)]
            }
            FeedsApiEvent::TimelineChangesFetched { changes, seq } => {
                self.apply_timeline_changes_fetched(changes, seq, entries_first, entries_limit)
            }
            FeedsApiEvent::FeedUnsubscribed { url } => {
                self.feeds.feed_unsubscribed(&url);
                self.shell.filter.update_categories(
                    &self.shell.categories,
                    Populate::Replace,
                    self.feeds.entries.entries(),
                );
                Vec::new()
            }
            FeedsApiEvent::SubscriptionFetched {
                populate,
                subscription,
            } => self.apply_subscription_fetched(populate, subscription),
            FeedsApiEvent::EntriesFetched { populate, payload } => self.apply_entries_fetched(
                request_seq,
                populate,
                payload,
                entries_first,
                entries_limit,
            ),
        }
    }

    fn apply_subscription_fetched(
        &mut self,
        populate: Populate,
        subscription: payload::SubscriptionPayload,
    ) -> Vec<Operation> {
        let mut operations = Vec::new();

        if subscription.feeds.page_info.has_next_page {
            operations.push(Operation::FetchSubscription {
                populate: Populate::Append,
                after: subscription.feeds.page_info.end_cursor.clone(),
                first: subscription.feeds.nodes.len().try_into().unwrap_or(0),
            });
        }
        self.feeds
            .subscription
            .update_subscription(populate, subscription);

        operations
    }

    fn apply_entries_fetched(
        &mut self,
        request_seq: RequestSequence,
        populate: Populate,
        payload: payload::TimelineEntryConnection,
        entries_first: i64,
        entries_limit: usize,
    ) -> Vec<Operation> {
        if !self.feeds.accept_entry_response(request_seq) {
            debug!(request_seq, ?populate, "ignore stale entries response");
            return Vec::new();
        }

        let page_info = payload.page_info.clone();
        let loaded_after_response =
            self.entries_loaded_after_response(populate, payload.nodes.len());
        let next_first = next_entries_first(entries_limit, entries_first, loaded_after_response);
        let mut operations = Vec::new();

        self.shell.filter.update_categories(
            &self.shell.categories,
            populate,
            payload.nodes.iter().map(|entry| &entry.entry),
        );
        if page_info.has_next_page && next_first > 0 {
            operations.push(Operation::FetchEntries {
                populate: Populate::Append,
                after: page_info.end_cursor,
                first: next_first,
            });
        }
        // Bootstrap contract: the seq of the first(Replace) page is the sync
        // starting point. Drift within later pages heals through idempotent
        // change application
        if populate == Populate::Replace {
            self.feeds.set_timeline_seq(payload.seq);
        }
        self.feeds
            .entries
            .update_entries_with_limit(populate, payload, entries_limit);
        operations
    }

    /// Apply synced timeline changes to the local timeline.
    fn apply_timeline_changes_fetched(
        &mut self,
        changes: Vec<payload::TimelineChange>,
        seq: i64,
        entries_first: i64,
        entries_limit: usize,
    ) -> Vec<Operation> {
        // A seq going backwards means our sync position is invalid
        // (e.g. the server database was recreated): bootstrap from the window
        if seq < self.feeds.timeline_seq() {
            warn!(
                seq,
                current = self.feeds.timeline_seq(),
                "timeline seq went backwards; bootstrap from the window"
            );
            self.feeds.set_timeline_seq(0);
            return vec![FeedsComponent::reload_entries(entries_first)];
        }

        self.feeds.entries.apply_changes(changes, entries_limit);
        self.feeds.set_timeline_seq(seq);
        self.shell.filter.update_categories(
            &self.shell.categories,
            Populate::Replace,
            self.feeds.entries.entries(),
        );
        Vec::new()
    }

    pub(in crate::application) fn apply_github_api_event(
        &mut self,
        event: GitHubApiEvent,
    ) -> Vec<Operation> {
        match event {
            GitHubApiEvent::NotificationsFetched {
                notifications,
                populate,
            } => {
                let mut operations = Vec::new();
                let contexts = self
                    .github
                    .notifications
                    .update_notifications(populate, notifications);
                if !contexts.is_empty() {
                    operations.push(Operation::FetchGitHubNotificationDetails { contexts });
                }
                if let Some(operation) = self.github.fetch_next_notifications_if_needed() {
                    operations.push(operation);
                }
                if populate == Populate::Replace {
                    self.shell.filter.clear_gh_notifications_categories();
                }
                operations
            }
            GitHubApiEvent::IssueFetched {
                notification_id,
                issue,
            } => {
                if let Some(notification) = self.github.notifications.update_issue(
                    notification_id,
                    issue,
                    &self.shell.categories,
                ) {
                    let categories = notification.categories().cloned();
                    self.shell.filter.update_gh_notification_categories(
                        &self.shell.categories,
                        Populate::Append,
                        categories,
                    );
                }
                Vec::new()
            }
            GitHubApiEvent::PullRequestFetched {
                notification_id,
                pull_request,
            } => {
                if let Some(notification) = self.github.notifications.update_pull_request(
                    notification_id,
                    pull_request,
                    &self.shell.categories,
                ) {
                    let categories = notification.categories().cloned();
                    self.shell.filter.update_gh_notification_categories(
                        &self.shell.categories,
                        Populate::Append,
                        categories,
                    );
                }
                Vec::new()
            }
            GitHubApiEvent::NotificationMarkedAsDone { notification_id } => {
                self.github.notifications.marked_as_done(notification_id);
                Vec::new()
            }
            GitHubApiEvent::ThreadUnsubscribed => Vec::new(),
        }
    }

    fn entries_loaded_after_response(&self, populate: Populate, response_len: usize) -> usize {
        match populate {
            Populate::Replace => response_len,
            Populate::Append => self.feeds.entries.loaded_count() + response_len,
        }
    }
}

fn next_entries_first(
    entries_limit: usize,
    entries_first: i64,
    loaded_after_response: usize,
) -> i64 {
    let remaining = entries_limit.saturating_sub(loaded_after_response);
    let page_size = usize::try_from(entries_first).unwrap_or(0);

    remaining.min(page_size).try_into().unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use core::assert_matches;
    use synd_feed::{entry::EntryId, types::FeedUrl};

    use crate::{application::Features, config::Categories, ui::theme::Theme};

    use super::*;

    fn app_component() -> AppComponent {
        AppComponent::new(
            &Features::default(),
            Theme::default(),
            Categories::default_toml(),
            false,
        )
    }

    fn entry(index: usize) -> payload::Entry {
        payload::Entry {
            id: EntryId::parse(format!("synd:entry:v1:{index:064x}")).unwrap(),
            title: Some(format!("entry-{index}")),
            published: None,
            updated: None,
            website_url: None,
            summary: None,
            feed: payload::FeedMeta {
                title: Some("feed".to_owned()),
                url: FeedUrl::parse("https://example.com/feed.xml").unwrap(),
                requirement: None,
                category: None,
            },
        }
    }

    #[test]
    fn feed_subscription_editor_emits_subscribe_operation() {
        let mut component = app_component();

        let operations = component.apply_feed_subscription_editor_closed(
            "MUST rust https://example.ymgyt.io/atom.xml interval:30m",
        );

        let [Operation::SubscribeFeed { input }] = operations.as_slice() else {
            panic!("expected SubscribeFeed operation");
        };
        assert_eq!(input.url.as_ref(), "https://example.ymgyt.io/atom.xml");
        assert_eq!(
            input
                .crawl_policy
                .as_ref()
                .and_then(|policy| policy.polling.interval_seconds),
            Some(1800)
        );
    }

    #[test]
    fn feed_event_timeline_changed_schedules_timeline_sync() {
        let mut component = app_component();
        let event = payload::FeedEvent::TimelineChanged(payload::TimelineChangeEvent {
            changed_at: "2026-06-13T00:00:00Z".to_owned(),
            affected_feeds: Some(vec![
                FeedUrl::parse("https://example.com/feed.xml").unwrap(),
            ]),
        });

        let operations = component.apply_feed_event(event);

        assert_matches!(operations.as_slice(), [Operation::ScheduleTimelineSync]);
        // Hints are coalesced until the debounced sync runs
        assert!(
            component
                .apply_feed_event(payload::FeedEvent::TimelineChanged(
                    payload::TimelineChangeEvent {
                        changed_at: "2026-06-13T00:00:01Z".to_owned(),
                        affected_feeds: None,
                    }
                ))
                .is_empty()
        );
    }

    #[test]
    fn timeline_changes_apply_upsert_and_remove_in_display_order() {
        let mut component = app_component();
        let upsert = |index: usize| payload::TimelineChange::Upsert {
            timeline_entry: Box::new(payload::TimelineEntry {
                order_time: chrono::DateTime::parse_from_rfc3339(&format!(
                    "2026-06-{index:02}T00:00:00Z"
                ))
                .unwrap()
                .to_utc(),
                entry: entry(index),
            }),
        };

        let operations = component.apply_timeline_changes_fetched(
            vec![upsert(1), upsert(3), upsert(2)],
            3,
            20,
            100,
        );
        assert!(operations.is_empty());
        assert_eq!(component.feeds.timeline_seq(), 3);
        let titles = component
            .feeds
            .entries
            .entries()
            .map(|entry| entry.title.clone().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(titles, ["entry-3", "entry-2", "entry-1"]);

        let operations = component.apply_timeline_changes_fetched(
            vec![payload::TimelineChange::Remove {
                entry_id: entry(2).id,
            }],
            4,
            20,
            100,
        );
        assert!(operations.is_empty());
        let titles = component
            .feeds
            .entries
            .entries()
            .map(|entry| entry.title.clone().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(titles, ["entry-3", "entry-1"]);
        assert_eq!(component.feeds.timeline_seq(), 4);
    }

    #[test]
    fn timeline_seq_going_backwards_falls_back_to_window_bootstrap() {
        let mut component = app_component();
        component.feeds.set_timeline_seq(10);

        let operations = component.apply_timeline_changes_fetched(Vec::new(), 3, 20, 100);

        assert_matches!(
            operations.as_slice(),
            [Operation::FetchEntries {
                populate: Populate::Replace,
                after: None,
                first: 20,
            }]
        );
        assert_eq!(component.feeds.timeline_seq(), 0);
    }
}
