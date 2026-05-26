use std::collections::{HashMap, HashSet};

use synd_feed::types::FeedUrl;

use crate::{
    client::synd_api::payload,
    operation::Operation,
    types::FeedRefreshStatus,
    ui::widgets::{
        entries::EntriesWidget,
        subscription::{SubscriptionWidget, UnsubscribeSelection},
    },
};
use url::Url;

use super::super::{
    Direction, FeedRefreshPollKey, Populate, RequestSequence, TimelineInvalidationState,
    input_parser::InputParser,
};

/// Feed timeline, subscription, refresh, and invalidation state machine.
pub(crate) struct FeedsComponent {
    pub(crate) subscription: SubscriptionWidget,
    pub(crate) entries: EntriesWidget,
    entry_fetches: EntryFetches,
    refreshes: FeedRefreshes,
    timeline: TimelineInvalidation,
}

impl FeedsComponent {
    pub(super) fn new() -> Self {
        Self {
            subscription: SubscriptionWidget::new(),
            entries: EntriesWidget::new(),
            entry_fetches: EntryFetches::new(),
            refreshes: FeedRefreshes::new(),
            timeline: TimelineInvalidation::new(),
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

    pub(in crate::application) fn open_unsubscribe_popup(&mut self) -> bool {
        if self.subscription.selected_feed().is_none() {
            return false;
        }
        self.subscription.toggle_unsubscribe_popup(true);
        true
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
                tracing::warn!("Try to open invalid feed url: {feed_website_url} {err}");
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
                tracing::warn!("Try to open/browse invalid entry url: {entry_website_url} {err}");
                None
            }
        }
    }

    pub(in crate::application) fn feed_subscribed(
        &mut self,
        url: FeedUrl,
        payload: payload::SubscribeFeedPayload,
        feeds_first: i64,
        entries_first: i64,
        refresh_poll_attempts: u16,
    ) -> Vec<Operation> {
        let mut operations = vec![
            Self::reload_subscription(feeds_first),
            Self::reload_entries(entries_first),
        ];

        if let Some(request_id) = payload.request_id
            && let Some(operation) =
                self.schedule_refresh_poll_if_needed(url, request_id, refresh_poll_attempts)
        {
            operations.push(operation);
        }

        operations
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

        let refresh_status = FeedRefreshStatus::from_refresh_receipt(&payload);
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
        entries_first: i64,
    ) -> Option<Vec<Operation>> {
        let poll_key = FeedRefreshPollKey::new(url.clone(), request_id.clone());
        if !self.refreshes.contains_poll(&poll_key) {
            return None;
        }

        let refresh_status = FeedRefreshStatus::from(status);
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
            self.refreshes.remove_poll(&poll_key);
            Some(vec![
                Self::reload_subscription(feeds_first),
                Self::reload_entries(entries_first),
            ])
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

/// Debounced timeline invalidation state.
struct TimelineInvalidation {
    state: TimelineInvalidationState,
    active_refetch: Option<RequestSequence>,
}

impl TimelineInvalidation {
    fn new() -> Self {
        Self {
            state: TimelineInvalidationState::Clean,
            active_refetch: None,
        }
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

impl TimelineInvalidation {
    fn mark_dirty(&mut self) -> Option<Operation> {
        match self.state {
            TimelineInvalidationState::Clean => {
                self.state = TimelineInvalidationState::DirtyWaiting;
                Some(Operation::ScheduleTimelineReload)
            }
            TimelineInvalidationState::Refetching => {
                self.state = TimelineInvalidationState::DirtyWhileRefetching;
                None
            }
            TimelineInvalidationState::DirtyWaiting
            | TimelineInvalidationState::DirtyWhileRefetching => None,
        }
    }

    fn should_refetch(&self) -> bool {
        self.state == TimelineInvalidationState::DirtyWaiting
    }

    fn skip_refetch(&mut self) {
        self.state = TimelineInvalidationState::Clean;
    }

    fn start_refetch(&mut self, request_seq: RequestSequence) {
        self.active_refetch = Some(request_seq);
        if self.state != TimelineInvalidationState::DirtyWhileRefetching {
            self.state = TimelineInvalidationState::Refetching;
        }
    }

    fn is_active_refetch(&self, request_seq: RequestSequence) -> bool {
        self.active_refetch == Some(request_seq)
    }

    fn complete_refetch(&mut self, request_seq: RequestSequence) -> Option<Operation> {
        if self.active_refetch != Some(request_seq) {
            return None;
        }

        self.active_refetch = None;
        match self.state {
            TimelineInvalidationState::Refetching => {
                self.state = TimelineInvalidationState::Clean;
                None
            }
            TimelineInvalidationState::DirtyWhileRefetching => {
                self.state = TimelineInvalidationState::DirtyWaiting;
                Some(Operation::ScheduleTimelineReload)
            }
            TimelineInvalidationState::Clean | TimelineInvalidationState::DirtyWaiting => None,
        }
    }

    fn fail_refetch(&mut self, request_seq: RequestSequence) -> Option<Operation> {
        if self.active_refetch != Some(request_seq) {
            return None;
        }

        self.active_refetch = None;
        self.state = TimelineInvalidationState::DirtyWaiting;
        Some(Operation::ScheduleTimelineReload)
    }

    #[cfg(feature = "integration")]
    fn has_pending_background_work(&self) -> bool {
        self.state != TimelineInvalidationState::Clean
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

    pub(in crate::application) fn should_refetch_timeline(&self) -> bool {
        self.timeline.should_refetch()
    }

    pub(in crate::application) fn skip_timeline_refetch(&mut self) {
        self.timeline.skip_refetch();
    }

    pub(in crate::application) fn start_timeline_refetch(&mut self, request_seq: RequestSequence) {
        self.timeline.start_refetch(request_seq);
    }

    pub(in crate::application) fn is_active_timeline_refetch(
        &self,
        request_seq: RequestSequence,
    ) -> bool {
        self.timeline.is_active_refetch(request_seq)
    }

    pub(in crate::application) fn complete_timeline_refetch(
        &mut self,
        request_seq: RequestSequence,
    ) -> Option<Operation> {
        self.timeline.complete_refetch(request_seq)
    }

    pub(in crate::application) fn fail_timeline_refetch(
        &mut self,
        request_seq: RequestSequence,
    ) -> Option<Operation> {
        self.timeline.fail_refetch(request_seq)
    }

    #[cfg(feature = "integration")]
    pub(in crate::application) fn has_pending_short_background_work(&self) -> bool {
        !self.refreshes.polls.is_empty() || self.timeline.has_pending_background_work()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn assert_schedule_timeline_reload(operation: Option<Operation>) {
        assert!(matches!(operation, Some(Operation::ScheduleTimelineReload)));
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
                20,
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
                20,
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
                20,
            )
            .expect("active poll should accept status response");

        assert!(matches!(
            operations.as_slice(),
            [
                Operation::FetchSubscription { first: 10, .. },
                Operation::FetchEntries { first: 20, .. }
            ]
        ));
        assert!(
            feeds
                .refresh_poll_elapsed(url, "refresh-1".to_owned(), 2)
                .is_none()
        );
    }

    #[test]
    fn timeline_change_while_refetching_schedules_another_reload_on_completion() {
        let mut timeline = TimelineInvalidation::new();

        assert_schedule_timeline_reload(timeline.mark_dirty());
        assert!(timeline.should_refetch());
        timeline.start_refetch(1);
        assert!(timeline.mark_dirty().is_none());

        assert_schedule_timeline_reload(timeline.complete_refetch(1));
        assert!(timeline.should_refetch());
    }

    #[test]
    fn timeline_refetch_failure_reschedules_reload() {
        let mut timeline = TimelineInvalidation::new();

        assert_schedule_timeline_reload(timeline.mark_dirty());
        timeline.start_refetch(1);

        assert_schedule_timeline_reload(timeline.fail_refetch(1));
        assert!(timeline.should_refetch());
    }

    #[test]
    fn timeline_refetch_ignores_unrelated_request() {
        let mut timeline = TimelineInvalidation::new();

        assert_schedule_timeline_reload(timeline.mark_dirty());
        timeline.start_refetch(1);

        assert!(timeline.complete_refetch(2).is_none());
        assert!(timeline.fail_refetch(2).is_none());
    }
}
