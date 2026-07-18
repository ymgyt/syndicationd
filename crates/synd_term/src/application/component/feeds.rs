use std::collections::HashSet;

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

use super::super::{Direction, Populate, RequestSequence, input_parser::InputParser};

/// Feed timeline, subscription, and sync state.
pub(crate) struct FeedsComponent {
    pub(crate) subscription: SubscriptionWidget,
    pub(crate) entries: EntriesWidget,
    entry_fetches: EntryFetches,
    timeline: TimelineSync,
}

impl FeedsComponent {
    pub(super) fn new() -> Self {
        Self {
            subscription: SubscriptionWidget::new(),
            entries: EntriesWidget::new(),
            entry_fetches: EntryFetches::new(),
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

    pub(in crate::application) fn feed_unsubscribed(&mut self, url: &FeedUrl) {
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

impl FeedsComponent {
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
        self.timeline.scheduled
    }
}
