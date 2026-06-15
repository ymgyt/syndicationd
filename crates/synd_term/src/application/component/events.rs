use synd_client::payload;
use tracing::debug;

use crate::{
    application::{Populate, RequestSequence, input_parser::InputParser},
    auth::AuthenticationProvider,
    event::{FeedsApiEvent, GitHubApiEvent},
    operation::Operation,
    ui::widgets::filter::Filterer,
};
use synd_auth::device_flow::DeviceAuthorizationResponse;
use synd_feed::types::FeedUrl;
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
        self.shell.request_render();

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
                self.shell.request_render();
                Vec::new()
            }
            Ok(input) => vec![Operation::SubscribeFeed { input }],
            Err(err) => {
                self.shell.prompt.set_error_message(err.to_string());
                self.shell.request_render();
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
                self.shell.request_render();
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
        feeds_first: i64,
        entries_first: i64,
    ) -> Vec<Operation> {
        match event {
            payload::FeedEvent::FeedSubscribed(event) => {
                debug!(url = %event.url, request_id = %event.request_id, "feed subscribed");
                self.shell.request_render();
                vec![
                    FeedsComponent::reload_subscription(feeds_first),
                    FeedsComponent::reload_entries(entries_first),
                ]
            }
            payload::FeedEvent::SubscriptionChanged(event) => {
                debug!(url = %event.url, request_id = %event.request_id, "feed subscription changed");
                self.shell.request_render();
                vec![
                    FeedsComponent::reload_subscription(feeds_first),
                    FeedsComponent::reload_entries(entries_first),
                ]
            }
            payload::FeedEvent::FeedUnsubscribed(event) => {
                debug!(url = %event.url, request_id = %event.request_id, "feed unsubscribed");
                self.feeds.feed_unsubscribed(&event.url);
                self.shell.filter.update_categories(
                    &self.shell.categories,
                    Populate::Replace,
                    self.feeds.entries.entries(),
                );
                self.shell.request_render();
                vec![FeedsComponent::reload_subscription(feeds_first)]
            }
            payload::FeedEvent::TimelineChanged(event) => self.apply_timeline_changed(&event),
            payload::FeedEvent::CrawlJobEnqueued(event) => {
                debug!(url = %event.url, "crawl job enqueued");
                Vec::new()
            }
            payload::FeedEvent::CrawlJobStarted(event) => {
                debug!(url = %event.url, "crawl job started");
                Vec::new()
            }
            payload::FeedEvent::CrawlJobFinished(event) => {
                debug!(
                    url = %event.url,
                    http_status = ?event.http_status,
                    error = ?event.error,
                    "crawl job finished"
                );
                Vec::new()
            }
            payload::FeedEvent::FeedDiscovered(event) => {
                debug!(url = %event.url, "feed discovered");
                Vec::new()
            }
            payload::FeedEvent::FeedChanged(event) => {
                debug!(url = %event.url, "feed changed");
                Vec::new()
            }
            payload::FeedEvent::EntryDiscovered(event) => {
                debug!(url = %event.url, "entry discovered");
                Vec::new()
            }
            payload::FeedEvent::EntryChanged(event) => {
                debug!(url = %event.url, "entry changed");
                Vec::new()
            }
            payload::FeedEvent::FeedSubscribeRejected(event) => {
                self.shell.prompt.set_error_message(event.reason);
                self.shell.request_render();
                Vec::new()
            }
            payload::FeedEvent::FeedUnsubscribeRejected(event) => {
                self.shell.prompt.set_error_message(event.reason);
                self.shell.request_render();
                Vec::new()
            }
        }
    }

    pub(in crate::application) fn apply_feed_event_subscription_interrupted(
        &mut self,
        feeds_first: i64,
        entries_first: i64,
    ) -> Vec<Operation> {
        self.shell.request_render();
        vec![Operation::ScheduleFeedViewReload {
            feeds_first,
            entries_first,
        }]
    }

    pub(in crate::application) fn apply_feed_view_reload_debounced(
        &mut self,
        feeds_first: i64,
        entries_first: i64,
    ) -> Vec<Operation> {
        self.shell.request_render();
        vec![
            Operation::FetchSubscription {
                populate: Populate::Replace,
                after: None,
                first: feeds_first,
            },
            FeedsComponent::reload_entries(entries_first),
        ]
    }

    pub(in crate::application) fn mark_timeline_dirty(&mut self) -> Vec<Operation> {
        self.feeds.mark_timeline_dirty().into_iter().collect()
    }

    pub(in crate::application) fn apply_feed_refresh_poll_elapsed(
        &mut self,
        url: FeedUrl,
        request_id: String,
        remaining: u16,
    ) -> Vec<Operation> {
        let operation = self.feeds.refresh_poll_elapsed(url, request_id, remaining);
        if operation.is_some() {
            self.shell.request_render();
        }
        operation.into_iter().collect()
    }

    pub(in crate::application) fn apply_feed_view_sync_elapsed(
        &mut self,
        feeds_first: i64,
        entries_first: i64,
    ) -> Vec<Operation> {
        self.shell.request_render();
        vec![
            Operation::FetchSubscription {
                populate: Populate::Replace,
                after: None,
                first: feeds_first,
            },
            FeedsComponent::reload_entries(entries_first),
            Operation::ScheduleFeedViewSync,
        ]
    }

    pub(in crate::application) fn apply_timeline_reload_debounced(
        &mut self,
        entries_first: i64,
    ) -> Vec<Operation> {
        if !self.feeds.should_refetch_timeline() {
            return Vec::new();
        }
        if entries_first <= 0 {
            self.feeds.skip_timeline_refetch();
            return Vec::new();
        }
        vec![Operation::RefetchTimelineEntries {
            populate: Populate::Replace,
            after: None,
            first: entries_first,
        }]
    }

    pub(in crate::application) fn apply_timeline_refetch_started(
        &mut self,
        request_seq: RequestSequence,
    ) {
        self.feeds.start_timeline_refetch(request_seq);
    }

    pub(in crate::application) fn apply_entry_fetch_started(
        &mut self,
        request_seq: RequestSequence,
        populate: Populate,
    ) {
        self.feeds.start_entry_fetch(request_seq, populate);
    }

    pub(in crate::application) fn apply_feed_refresh_requested(
        &mut self,
        request_seq: RequestSequence,
        url: FeedUrl,
    ) {
        self.feeds.track_refresh_request(request_seq, url);
    }

    pub(in crate::application) fn apply_synd_api_error(
        &mut self,
        request_seq: RequestSequence,
    ) -> Vec<Operation> {
        self.feeds.forget_refresh_request(request_seq);
        self.feeds.forget_entry_fetch(request_seq);
        self.feeds
            .fail_timeline_refetch(request_seq)
            .into_iter()
            .collect()
    }

    pub(in crate::application) fn apply_feed_refresh_poll_error(
        &mut self,
        url: FeedUrl,
        request_id: String,
    ) -> bool {
        self.feeds.refresh_poll_failed(url, request_id)
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
        refresh_poll_attempts: u16,
    ) -> Vec<Operation> {
        match event {
            FeedsApiEvent::FeedSubscribed { url, payload } => {
                let operations = FeedsComponent::feed_subscribed(
                    url,
                    payload,
                    feeds_first,
                    entries_first,
                    refresh_poll_attempts,
                );
                self.shell.request_render();
                operations
            }
            FeedsApiEvent::FeedRefreshAccepted { url, payload } => {
                let Some(operations) = self.feeds.feed_refresh_accepted(
                    request_seq,
                    url.clone(),
                    payload,
                    feeds_first,
                    refresh_poll_attempts,
                ) else {
                    return Vec::new();
                };
                debug!(%url, "refresh request accepted");
                self.shell.request_render();
                operations
            }
            FeedsApiEvent::FeedRefreshStatusFetched {
                url,
                request_id,
                remaining,
                status,
            } => {
                let Some(operations) = self.feeds.refresh_status_fetched(
                    url,
                    request_id,
                    remaining,
                    status,
                    feeds_first,
                    entries_first,
                ) else {
                    return Vec::new();
                };
                self.shell.request_render();
                operations
            }
            FeedsApiEvent::FeedUnsubscribed { url } => {
                self.feeds.feed_unsubscribed(&url);
                self.shell.filter.update_categories(
                    &self.shell.categories,
                    Populate::Replace,
                    self.feeds.entries.entries(),
                );
                self.shell.request_render();
                Vec::new()
            }
            FeedsApiEvent::SubscriptionFetched {
                populate,
                subscription,
            } => self.apply_subscription_fetched(populate, subscription, entries_first),
            FeedsApiEvent::EntriesFetched { populate, payload } => self.apply_entries_fetched(
                request_seq,
                populate,
                payload,
                entries_first,
                entries_limit,
            ),
            FeedsApiEvent::InitialFeedViewFetched { payload } => self
                .apply_initial_feed_view_fetched(
                    request_seq,
                    payload,
                    feeds_first,
                    entries_first,
                    entries_limit,
                ),
        }
    }

    fn apply_subscription_fetched(
        &mut self,
        populate: Populate,
        subscription: payload::SubscriptionPayload,
        entries_first: i64,
    ) -> Vec<Operation> {
        let has_snapshot = subscription
            .feeds
            .nodes
            .iter()
            .any(|feed| feed.feed.is_some());
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
        if populate == Populate::Replace && self.feeds.entries.count() == 0 && has_snapshot {
            operations.push(Operation::FetchEntries {
                populate: Populate::Replace,
                after: None,
                first: entries_first,
            });
        }

        self.shell.request_render();
        operations
    }

    fn apply_entries_fetched(
        &mut self,
        request_seq: RequestSequence,
        populate: Populate,
        payload: payload::FetchEntriesPayload,
        entries_first: i64,
        entries_limit: usize,
    ) -> Vec<Operation> {
        let is_timeline_refetch = self.feeds.is_active_timeline_refetch(request_seq);
        if !self.feeds.accept_entry_response(request_seq) {
            debug!(request_seq, ?populate, "ignore stale entries response");
            if is_timeline_refetch {
                return self
                    .feeds
                    .fail_timeline_refetch(request_seq)
                    .into_iter()
                    .collect();
            }
            return Vec::new();
        }

        let page_info = payload.page_info.clone();
        let loaded_after_response =
            self.entries_loaded_after_response(populate, payload.entries.len());
        let next_first = next_entries_first(entries_limit, entries_first, loaded_after_response);
        let mut operations = Vec::new();

        self.shell.filter.update_categories(
            &self.shell.categories,
            populate,
            payload.entries.as_slice(),
        );
        let should_fetch_next_page = page_info.has_next_page && next_first > 0;
        if should_fetch_next_page {
            if is_timeline_refetch {
                operations.push(Operation::RefetchTimelineEntries {
                    populate: Populate::Append,
                    after: page_info.end_cursor,
                    first: next_first,
                });
            } else {
                operations.push(Operation::FetchEntries {
                    populate: Populate::Append,
                    after: page_info.end_cursor,
                    first: next_first,
                });
            }
        }
        self.feeds
            .entries
            .update_entries_with_limit(populate, payload, entries_limit);
        if !should_fetch_next_page
            && let Some(operation) = self.feeds.complete_timeline_refetch(request_seq)
        {
            operations.push(operation);
        }
        self.shell.request_render();
        operations
    }

    fn apply_initial_feed_view_fetched(
        &mut self,
        request_seq: RequestSequence,
        payload: payload::InitialFeedViewPayload,
        feeds_first: i64,
        entries_first: i64,
        entries_limit: usize,
    ) -> Vec<Operation> {
        let (subscription, entries) = payload.into_parts();
        let mut operations = Vec::new();

        if let Some(subscription) = subscription {
            if subscription.feeds.page_info.has_next_page {
                operations.push(Operation::FetchSubscription {
                    populate: Populate::Append,
                    after: subscription.feeds.page_info.end_cursor.clone(),
                    first: subscription.feeds.nodes.len().try_into().unwrap_or(0),
                });
            }
            self.feeds
                .subscription
                .update_subscription(Populate::Replace, subscription);
        } else {
            operations.push(Operation::FetchSubscription {
                populate: Populate::Replace,
                after: None,
                first: feeds_first,
            });
        }

        let accept_timeline = self.feeds.accept_entry_response(request_seq);
        if let Some(entries) = entries.filter(|_| accept_timeline) {
            let next_first =
                next_entries_first(entries_limit, entries_first, entries.entries.len());
            if entries.page_info.has_next_page && next_first > 0 {
                operations.push(Operation::FetchEntries {
                    populate: Populate::Append,
                    after: entries.page_info.end_cursor.clone(),
                    first: next_first,
                });
            }
            self.shell.filter.update_categories(
                &self.shell.categories,
                Populate::Replace,
                entries.entries.as_slice(),
            );
            self.feeds
                .entries
                .update_entries_with_limit(Populate::Replace, entries, entries_limit);
        } else if accept_timeline {
            operations.push(Operation::FetchEntries {
                populate: Populate::Replace,
                after: None,
                first: entries_first,
            });
        }

        self.shell.request_render();
        operations
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
                self.shell.request_render();
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
                self.shell.request_render();
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
                self.shell.request_render();
                Vec::new()
            }
            GitHubApiEvent::NotificationMarkedAsDone { notification_id } => {
                self.github.notifications.marked_as_done(notification_id);
                self.shell.request_render();
                Vec::new()
            }
            GitHubApiEvent::ThreadUnsubscribed { .. } => Vec::new(),
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
    use synd_client::payload::{ResponseCode, ResponseStatus};
    use synd_feed::types::FeedUrl;

    use crate::{
        application::Features, config::Categories, event::FeedsApiEvent, types::PageInfo,
        ui::theme::Theme,
    };

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

    fn entries_payload(entry_count: usize, has_next_page: bool) -> payload::FetchEntriesPayload {
        payload::FetchEntriesPayload {
            entries: (0..entry_count).map(entry).collect(),
            page_info: PageInfo {
                has_next_page,
                end_cursor: Some("cursor".to_owned()),
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
    fn feed_subscribed_response_reloads_subscription_and_entries() {
        let mut component = app_component();
        let url = FeedUrl::parse("https://example.com/feed.xml").unwrap();
        let payload = payload::SubscribeFeedPayload {
            status: ResponseStatus {
                code: ResponseCode::Ok,
            },
            url: url.clone(),
            request_id: "request-1".to_owned(),
        };

        let operations = component.apply_feeds_api_event(
            1,
            FeedsApiEvent::FeedSubscribed { url, payload },
            10,
            20,
            100,
            3,
        );

        assert_matches!(
            operations.as_slice(),
            [
                Operation::FetchSubscription {
                    populate: Populate::Replace,
                    after: None,
                    first: 10,
                },
                Operation::FetchEntries {
                    populate: Populate::Replace,
                    after: None,
                    first: 20,
                },
                Operation::ScheduleFeedViewReload {
                    feeds_first: 10,
                    entries_first: 20,
                },
            ]
        );
    }

    #[test]
    fn feed_event_subscription_interruption_schedules_delayed_feed_view_reload() {
        let mut component = app_component();

        let operations = component.apply_feed_event_subscription_interrupted(10, 20);

        assert_matches!(
            operations.as_slice(),
            [Operation::ScheduleFeedViewReload {
                feeds_first: 10,
                entries_first: 20,
            }]
        );

        let operations = component.apply_feed_view_reload_debounced(10, 20);
        assert_matches!(
            operations.as_slice(),
            [
                Operation::FetchSubscription {
                    populate: Populate::Replace,
                    after: None,
                    first: 10,
                },
                Operation::FetchEntries {
                    populate: Populate::Replace,
                    after: None,
                    first: 20,
                },
            ]
        );
    }

    #[test]
    fn feed_event_subscription_confirmation_reloads_subscription_and_entries() {
        let mut component = app_component();
        let event = payload::FeedEvent::SubscriptionChanged(payload::SubscriptionChangedEvent {
            request_id: "request-1".to_owned(),
            url: FeedUrl::parse("https://example.com/feed.xml").unwrap(),
        });

        let operations = component.apply_feed_event(event, 10, 20);

        assert_matches!(
            operations.as_slice(),
            [
                Operation::FetchSubscription {
                    populate: Populate::Replace,
                    after: None,
                    first: 10,
                },
                Operation::FetchEntries {
                    populate: Populate::Replace,
                    after: None,
                    first: 20,
                },
            ]
        );
    }

    #[test]
    fn timeline_changed_event_schedules_debounced_refetch() {
        let mut component = app_component();
        let event = payload::TimelineChangeEvent {
            changed_at: "2026-06-13T00:00:00Z".to_owned(),
            affected_feeds: Some(vec![
                FeedUrl::parse("https://example.com/feed.xml").unwrap(),
            ]),
        };

        let operations = component.apply_timeline_changed(&event);

        assert_matches!(operations.as_slice(), [Operation::ScheduleTimelineReload]);

        let operations = component.apply_timeline_reload_debounced(20);

        assert_matches!(
            operations.as_slice(),
            [Operation::RefetchTimelineEntries {
                populate: Populate::Replace,
                after: None,
                first: 20,
            }]
        );
    }

    #[test]
    fn timeline_refetch_next_page_keeps_timeline_operation() {
        let mut component = app_component();
        component.apply_entry_fetch_started(1, Populate::Replace);
        component.apply_timeline_refetch_started(1);

        let operations = component.apply_feeds_api_event(
            1,
            FeedsApiEvent::EntriesFetched {
                populate: Populate::Replace,
                payload: entries_payload(1, true),
            },
            10,
            2,
            4,
            3,
        );

        assert!(component.feeds.is_active_timeline_refetch(1));
        assert_eq!(operations.len(), 1);
        let Operation::RefetchTimelineEntries {
            populate,
            after,
            first,
        } = &operations[0]
        else {
            panic!("expected RefetchTimelineEntries");
        };
        assert_eq!(*populate, Populate::Append);
        assert_eq!(after.as_deref(), Some("cursor"));
        assert_eq!(*first, 2);
    }

    #[test]
    fn timeline_refetch_completes_after_last_page() {
        let mut component = app_component();
        component.apply_entry_fetch_started(1, Populate::Replace);
        component.apply_timeline_refetch_started(1);
        assert!(component.mark_timeline_dirty().is_empty());

        let operations = component.apply_feeds_api_event(
            1,
            FeedsApiEvent::EntriesFetched {
                populate: Populate::Replace,
                payload: entries_payload(1, false),
            },
            10,
            2,
            4,
            3,
        );

        assert!(!component.feeds.is_active_timeline_refetch(1));
        assert!(component.feeds.should_refetch_timeline());
        assert_matches!(operations.as_slice(), [Operation::ScheduleTimelineReload]);
    }

    #[test]
    fn timeline_refetch_preserves_dirty_state_across_next_page_start() {
        let mut component = app_component();
        component.apply_entry_fetch_started(1, Populate::Replace);
        component.apply_timeline_refetch_started(1);
        assert!(component.mark_timeline_dirty().is_empty());

        let operations = component.apply_feeds_api_event(
            1,
            FeedsApiEvent::EntriesFetched {
                populate: Populate::Replace,
                payload: entries_payload(1, true),
            },
            10,
            2,
            4,
            3,
        );
        assert_matches!(
            operations.as_slice(),
            [Operation::RefetchTimelineEntries { .. }]
        );

        component.apply_entry_fetch_started(2, Populate::Append);
        component.apply_timeline_refetch_started(2);
        let operations = component.apply_feeds_api_event(
            2,
            FeedsApiEvent::EntriesFetched {
                populate: Populate::Append,
                payload: entries_payload(1, false),
            },
            10,
            2,
            4,
            3,
        );

        assert_matches!(operations.as_slice(), [Operation::ScheduleTimelineReload]);
        assert!(component.feeds.should_refetch_timeline());
    }

    #[test]
    fn stale_entries_response_is_ignored_after_newer_replace_starts() {
        let mut component = app_component();
        component.apply_entry_fetch_started(1, Populate::Append);
        assert_matches!(
            component.mark_timeline_dirty().as_slice(),
            [Operation::ScheduleTimelineReload]
        );
        let operations = component.apply_timeline_reload_debounced(2);
        assert_matches!(
            operations.as_slice(),
            [Operation::RefetchTimelineEntries { .. }]
        );
        component.apply_entry_fetch_started(2, Populate::Replace);
        component.apply_timeline_refetch_started(2);

        let operations = component.apply_feeds_api_event(
            1,
            FeedsApiEvent::EntriesFetched {
                populate: Populate::Append,
                payload: entries_payload(1, true),
            },
            10,
            2,
            4,
            3,
        );

        assert!(operations.is_empty());
        assert_eq!(component.feeds.entries.count(), 0);
    }

    #[test]
    fn stale_active_timeline_refetch_does_not_remain_pending() {
        let mut component = app_component();
        component.apply_entry_fetch_started(1, Populate::Replace);
        component.apply_timeline_refetch_started(1);
        component.apply_entry_fetch_started(2, Populate::Replace);

        let operations = component.apply_feeds_api_event(
            1,
            FeedsApiEvent::EntriesFetched {
                populate: Populate::Replace,
                payload: entries_payload(1, true),
            },
            10,
            2,
            4,
            3,
        );

        assert!(!component.feeds.is_active_timeline_refetch(1));
        assert!(component.feeds.should_refetch_timeline());
        assert_matches!(operations.as_slice(), [Operation::ScheduleTimelineReload]);
        assert_eq!(component.feeds.entries.count(), 0);
    }
}
