use std::collections::{HashMap, HashSet};

use synd_client::payload;
use synd_feed::types::FeedUrl;
use tracing::warn;
use url::Url;

use crate::{
    operation::Operation,
    ui::widgets::{
        entries::EntriesWidget,
        subscription::{SubscriptionWidget, UnsubscribeSelection},
    },
};

use super::super::{
    Direction, FeedRefreshPollKey, Populate, RequestSequence, input_parser::InputParser,
};

/// Feed timeline, subscription, refresh, and sync state.
pub(crate) struct FeedsComponent {
    pub(crate) subscription: SubscriptionWidget,
    pub(crate) entries: EntriesWidget,
    entry_fetches: EntryFetches,
    refreshes: FeedRefreshes,
    timeline: TimelineSync,
}

impl FeedsComponent {
    pub(super) fn new() -> Self {
        Self {
            subscription: SubscriptionWidget::new(),
            entries: EntriesWidget::new(),
            entry_fetches: EntryFetches::new(),
            refreshes: FeedRefreshes::new(),
            timeline: TimelineSync::new(),
        }
    }

    pub(in crate::application) fn move_subscription(&mut self, direction: Direction) {
        self.subscription.move_selection(direction);
    }

    pub(in crate::application) fn has_subscription(&self) -> bool {
        self.subscription.has_subscription()
    }

    pub(in crate::application) fn is_already_subscribed(&self, url: &FeedUrl) -> bool {
        self.subscription.is_already_subscribed(url)
    }

    pub(in crate::application) fn move_subscription_first(&mut self) {
        self.subscription.move_first();
    }

    pub(in crate::application) fn move_subscription_last(&mut self) {
        self.subscription.move_last();
    }

    pub(in crate::application) fn open_unsubscribe_popup(&mut self) {
        if self.subscription.selected_feed().is_none() {
            return;
        }
        self.subscription.toggle_unsubscribe_popup(true);
    }

    pub(in crate::application) fn is_unsubscribe_popup_open(&self) -> bool {
        self.subscription.unsubscribe_popup_selection().1.is_some()
    }

    pub(in crate::application) fn move_unsubscribe_popup_selection(
        &mut self,
        direction: Direction,
    ) {
        self.subscription
            .move_unsubscribe_popup_selection(direction);
    }

    pub(in crate::application) fn selected_unsubscribe_operation(&self) -> Option<Operation> {
        let (UnsubscribeSelection::Yes, Some(feed)) =
            self.subscription.unsubscribe_popup_selection()
        else {
            return None;
        };
        Some(Operation::UnsubscribeFeed {
            url: feed.url.clone(),
        })
    }

    pub(in crate::application) fn close_unsubscribe_popup(&mut self) {
        self.subscription.toggle_unsubscribe_popup(false);
    }

    pub(in crate::application) fn refresh_selected_feed(&self) -> Option<Operation> {
        self.subscription
            .selected_feed()
            .map(|feed| Operation::RefreshFeed {
                url: feed.url.clone(),
            })
    }

    pub(in crate::application) fn edit_selected_feed(&self) -> Option<Operation> {
        self.subscription
            .selected_feed()
            .map(|feed| Operation::OpenFeedEditionEditor {
                prompt: InputParser::edit_feed_prompt(feed),
            })
    }

    pub(in crate::application) fn open_selected_feed(&self) -> Option<Operation> {
        let feed_website_url = self.subscription.selected_feed()?.website_url.as_ref()?;
        match Url::parse(feed_website_url) {
            Ok(url) => Some(Operation::OpenBrowser { url }),
            Err(err) => {
                warn!("Try to open invalid feed url: {feed_website_url} {err}");
                None
            }
        }
    }

    pub(in crate::application) fn open_selected_entry(&self) -> Option<Operation> {
        self.selected_entry_url()
            .map(|url| Operation::OpenBrowser { url })
    }

    pub(in crate::application) fn browse_selected_entry(&self) -> Vec<Operation> {
        let Some(url) = self.selected_entry_url() else {
            return Vec::new();
        };
        vec![
            Operation::OpenTextBrowser { url },
            Operation::ForceRedrawTerminal,
        ]
    }

    fn selected_entry_url(&self) -> Option<Url> {
        let entry_website_url = self.entries.selected_entry_website_url()?;
        match Url::parse(entry_website_url) {
            Ok(url) => Some(url),
            Err(err) => {
                warn!("Try to open/browse invalid entry url: {entry_website_url} {err}");
                None
            }
        }
    }

    pub(in crate::application) fn feed_refresh_accepted(
        &mut self,
        request_seq: RequestSequence,
        url: FeedUrl,
        payload: payload::RefreshFeedPayload,
        feeds_first: i64,
        refresh_poll_attempts: u16,
    ) -> Option<Vec<Operation>> {
        if !self.refreshes.accept_request(request_seq) {
            return None;
        }

        let refresh_status = payload::RefreshStatus::from(&payload);
        self.subscription
            .update_refresh_status(&url, &refresh_status);

        let mut operations = vec![Self::reload_subscription(feeds_first)];
        if let Some(operation) =
            self.schedule_refresh_poll_if_needed(url, payload.request_id, refresh_poll_attempts)
        {
            operations.push(operation);
        }

        Some(operations)
    }

    pub(in crate::application) fn refresh_status_fetched(
        &mut self,
        url: FeedUrl,
        request_id: String,
        remaining: u16,
        status: payload::RefreshStatus,
        feeds_first: i64,
    ) -> Option<Vec<Operation>> {
        let poll_key = FeedRefreshPollKey::new(url.clone(), request_id.clone());
        if !self.refreshes.contains_poll(&poll_key) {
            return None;
        }

        let refresh_status = status;
        let current_request_id = refresh_status.request_id.clone();
        let is_current_request = current_request_id.as_deref() == Some(request_id.as_str());
        let is_active = refresh_status.is_active();
        self.subscription
            .update_refresh_status(&url, &refresh_status);

        if is_current_request && is_active && remaining > 1 {
            Some(vec![Operation::ScheduleFeedRefreshPoll {
                url,
                request_id,
                remaining: remaining.saturating_sub(1),
            }])
        } else if is_current_request || !is_active {
            // Refresh finished: reload the refresh status column. New entries
            // arrive through the timeline change push
            self.refreshes.remove_poll(&poll_key);
            Some(vec![Self::reload_subscription(feeds_first)])
        } else {
            self.refreshes.remove_poll(&poll_key);
            Some(Vec::new())
        }
    }

    pub(in crate::application) fn refresh_poll_elapsed(
        &self,
        url: FeedUrl,
        request_id: String,
        remaining: u16,
    ) -> Option<Operation> {
        let poll_key = FeedRefreshPollKey::new(url.clone(), request_id.clone());
        self.refreshes
            .contains_poll(&poll_key)
            .then_some(Operation::FetchFeedRefreshStatus {
                url,
                request_id,
                remaining,
            })
    }

    pub(in crate::application) fn refresh_poll_failed(
        &mut self,
        url: FeedUrl,
        request_id: String,
    ) -> bool {
        let poll_key = FeedRefreshPollKey::new(url, request_id);
        self.refreshes.remove_poll(&poll_key)
    }

    pub(in crate::application) fn feed_unsubscribed(&mut self, url: &FeedUrl) {
        self.refreshes.remove_for_url(url);
        self.subscription.remove_unsubscribed_feed(url);
        self.entries.remove_unsubscribed_entries(url);
    }

    pub(in crate::application) fn reload_subscription(first: i64) -> Operation {
        Operation::FetchSubscription {
            populate: Populate::Replace,
            after: None,
            first,
        }
    }

    pub(in crate::application) fn reload_entries(first: i64) -> Operation {
        Operation::FetchEntries {
            populate: Populate::Replace,
            after: None,
            first,
        }
    }

    pub(in crate::application) fn move_entry(&mut self, direction: Direction) {
        self.entries.move_selection(direction);
    }

    pub(in crate::application) fn move_entry_first(&mut self) {
        self.entries.move_first();
    }

    pub(in crate::application) fn move_entry_last(&mut self) {
        self.entries.move_last();
    }
}

/// Feed refresh request and polling state.
struct FeedRefreshes {
    requests: HashMap<RequestSequence, FeedUrl>,
    polls: HashSet<FeedRefreshPollKey>,
}

/// In-flight entry fetches and the request barrier for the current entries view.
struct EntryFetches {
    latest_replace_started_at: Option<RequestSequence>,
    active: HashSet<RequestSequence>,
}

impl EntryFetches {
    fn new() -> Self {
        Self {
            latest_replace_started_at: None,
            active: HashSet::new(),
        }
    }

    fn start(&mut self, request_seq: RequestSequence, populate: Populate) {
        self.active.insert(request_seq);
        if populate == Populate::Replace {
            self.latest_replace_started_at = Some(request_seq);
        }
    }

    fn accept_response(&mut self, request_seq: RequestSequence) -> bool {
        if !self.active.remove(&request_seq) {
            return false;
        }
        self.latest_replace_started_at
            .is_none_or(|barrier| request_seq >= barrier)
    }

    fn forget(&mut self, request_seq: RequestSequence) {
        self.active.remove(&request_seq);
    }
}

impl FeedRefreshes {
    fn new() -> Self {
        Self {
            requests: HashMap::new(),
            polls: HashSet::new(),
        }
    }
}

/// Cursor of the timeline change feed and its debounce state.
struct TimelineSync {
    /// Last change seq applied to the local timeline
    seq: i64,
    /// Whether a debounced sync is already scheduled
    scheduled: bool,
}

impl TimelineSync {
    fn new() -> Self {
        Self {
            seq: 0,
            scheduled: false,
        }
    }

    /// Coalesce change hints into one debounced sync.
    fn mark_dirty(&mut self) -> Option<Operation> {
        if self.scheduled {
            return None;
        }
        self.scheduled = true;
        Some(Operation::ScheduleTimelineSync)
    }

    fn debounce_elapsed(&mut self) -> Operation {
        self.scheduled = false;
        Operation::SyncTimeline { since: self.seq }
    }
}

impl FeedRefreshes {
    fn track_request(&mut self, request_seq: RequestSequence, url: FeedUrl) {
        self.requests.insert(request_seq, url);
    }

    fn accept_request(&mut self, request_seq: RequestSequence) -> bool {
        self.requests.remove(&request_seq).is_some()
    }

    fn remove_request(&mut self, request_seq: RequestSequence) {
        self.requests.remove(&request_seq);
    }

    fn insert_poll(&mut self, key: FeedRefreshPollKey) -> bool {
        self.polls.insert(key)
    }

    fn contains_poll(&self, key: &FeedRefreshPollKey) -> bool {
        self.polls.contains(key)
    }

    fn remove_poll(&mut self, key: &FeedRefreshPollKey) -> bool {
        self.polls.remove(key)
    }

    fn remove_for_url(&mut self, url: &FeedUrl) {
        self.requests.retain(|_, request_url| request_url != url);
        self.polls.retain(|key| &key.url != url);
    }
}

impl FeedsComponent {
    pub(in crate::application) fn track_refresh_request(
        &mut self,
        request_seq: RequestSequence,
        url: FeedUrl,
    ) {
        self.refreshes.track_request(request_seq, url);
    }

    pub(in crate::application) fn forget_refresh_request(&mut self, request_seq: RequestSequence) {
        self.refreshes.remove_request(request_seq);
    }

    pub(in crate::application) fn schedule_refresh_poll_if_needed(
        &mut self,
        url: FeedUrl,
        request_id: String,
        remaining: u16,
    ) -> Option<Operation> {
        let key = FeedRefreshPollKey::new(url.clone(), request_id.clone());
        self.refreshes
            .insert_poll(key)
            .then_some(Operation::ScheduleFeedRefreshPoll {
                url,
                request_id,
                remaining,
            })
    }

    pub(in crate::application) fn mark_timeline_dirty(&mut self) -> Option<Operation> {
        self.timeline.mark_dirty()
    }

    pub(in crate::application) fn start_entry_fetch(
        &mut self,
        request_seq: RequestSequence,
        populate: Populate,
    ) {
        self.entry_fetches.start(request_seq, populate);
    }

    pub(in crate::application) fn accept_entry_response(
        &mut self,
        request_seq: RequestSequence,
    ) -> bool {
        self.entry_fetches.accept_response(request_seq)
    }

    pub(in crate::application) fn forget_entry_fetch(&mut self, request_seq: RequestSequence) {
        self.entry_fetches.forget(request_seq);
    }

    pub(in crate::application) fn timeline_sync_debounced(&mut self) -> Operation {
        self.timeline.debounce_elapsed()
    }

    pub(in crate::application) fn timeline_seq(&self) -> i64 {
        self.timeline.seq
    }

    pub(in crate::application) fn set_timeline_seq(&mut self, seq: i64) {
        self.timeline.seq = seq;
    }

    /// Operation that syncs the timeline from the current seq immediately.
    pub(in crate::application) fn sync_timeline(&self) -> Operation {
        Operation::SyncTimeline {
            since: self.timeline.seq,
        }
    }

    #[cfg(feature = "integration")]
    pub(in crate::application) fn has_pending_short_background_work(&self) -> bool {
        !self.refreshes.polls.is_empty() || self.timeline.scheduled
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::assert_matches;

    fn feed_url() -> FeedUrl {
        FeedUrl::parse("https://example.com/feed.xml").unwrap()
    }

    fn refresh_status(
        request_id: Option<&str>,
        state: payload::RefreshStatusState,
    ) -> payload::RefreshStatus {
        payload::RefreshStatus {
            state,
            request_id: request_id.map(str::to_owned),
            last_attempt_at: None,
            last_success_at: None,
            last_failure_at: None,
            last_error_message: None,
        }
    }

    fn assert_schedule_poll(
        operation: Option<Operation>,
        expected_url: &FeedUrl,
        expected_request_id: &str,
        expected_remaining: u16,
    ) {
        let Some(Operation::ScheduleFeedRefreshPoll {
            url,
            request_id,
            remaining,
        }) = operation
        else {
            panic!("expected ScheduleFeedRefreshPoll");
        };
        assert_eq!(&url, expected_url);
        assert_eq!(request_id, expected_request_id);
        assert_eq!(remaining, expected_remaining);
    }

    #[test]
    fn duplicate_refresh_poll_is_not_scheduled_twice() {
        let mut feeds = FeedsComponent::new();
        let url = feed_url();

        assert_schedule_poll(
            feeds.schedule_refresh_poll_if_needed(url.clone(), "refresh-1".to_owned(), 3),
            &url,
            "refresh-1",
            3,
        );
        assert!(
            feeds
                .schedule_refresh_poll_if_needed(url, "refresh-1".to_owned(), 3)
                .is_none()
        );
    }

    #[test]
    fn stale_refresh_status_finishes_poll_without_reloading() {
        let mut feeds = FeedsComponent::new();
        let url = feed_url();
        feeds.schedule_refresh_poll_if_needed(url.clone(), "refresh-1".to_owned(), 3);

        let operations = feeds
            .refresh_status_fetched(
                url.clone(),
                "refresh-1".to_owned(),
                3,
                refresh_status(Some("refresh-2"), payload::RefreshStatusState::Running),
                10,
            )
            .expect("active poll should accept status response");

        assert!(operations.is_empty());
        assert!(
            feeds
                .refresh_poll_elapsed(url, "refresh-1".to_owned(), 2)
                .is_none()
        );
    }

    #[test]
    fn active_current_refresh_status_schedules_next_poll() {
        let mut feeds = FeedsComponent::new();
        let url = feed_url();
        feeds.schedule_refresh_poll_if_needed(url.clone(), "refresh-1".to_owned(), 3);

        let operations = feeds
            .refresh_status_fetched(
                url.clone(),
                "refresh-1".to_owned(),
                3,
                refresh_status(Some("refresh-1"), payload::RefreshStatusState::Running),
                10,
            )
            .expect("active poll should accept status response");

        assert_eq!(operations.len(), 1);
        let Operation::ScheduleFeedRefreshPoll {
            url: operation_url,
            request_id,
            remaining,
        } = &operations[0]
        else {
            panic!("expected ScheduleFeedRefreshPoll");
        };
        assert_eq!(operation_url, &url);
        assert_eq!(request_id, "refresh-1");
        assert_eq!(*remaining, 2);
    }

    #[test]
    fn completed_refresh_status_reloads_feed_view() {
        let mut feeds = FeedsComponent::new();
        let url = feed_url();
        feeds.schedule_refresh_poll_if_needed(url.clone(), "refresh-1".to_owned(), 3);

        let operations = feeds
            .refresh_status_fetched(
                url.clone(),
                "refresh-1".to_owned(),
                3,
                refresh_status(Some("refresh-1"), payload::RefreshStatusState::Idle),
                10,
            )
            .expect("active poll should accept status response");

        assert_matches!(
            operations.as_slice(),
            [Operation::FetchSubscription { first: 10, .. }]
        );
        assert!(
            feeds
                .refresh_poll_elapsed(url, "refresh-1".to_owned(), 2)
                .is_none()
        );
    }
}
