use synd_client::payload;
use synd_feed::types::FeedUrl;
use tracing::warn;
use url::Url;

use crate::{
    application::{Direction, Populate, input_parser::InputParser},
    operation::{Operation, Operations},
    ui::widgets::{
        entries::EntriesWidget,
        subscription::{SubscriptionWidget, UnsubscribeSelection},
    },
};

/// Timeline bootstrap and catch-up state.
#[derive(Debug)]
pub(crate) enum TimelineState {
    Uninitialized,
    FetchingWindow { base_seq: Option<i64> },
    CatchingUp { seq: i64, dirty: bool },
    Ready { seq: i64 },
}

/// Feed subscription and timeline application state.
pub(crate) struct FeedsComponent {
    pub(crate) subscription: SubscriptionWidget,
    pub(crate) entries: EntriesWidget,
    timeline: TimelineState,
}

impl FeedsComponent {
    pub(super) fn new() -> Self {
        Self {
            subscription: SubscriptionWidget::new(),
            entries: EntriesWidget::new(),
            timeline: TimelineState::Uninitialized,
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
        if self.subscription.selected_feed().is_some() {
            self.subscription.toggle_unsubscribe_popup(true);
        }
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
        Self::parse_browser_url(feed_website_url, "feed")
    }

    pub(in crate::application) fn open_selected_entry(&self) -> Option<Operation> {
        self.selected_entry_url()
            .map(|url| Operation::OpenBrowser { url })
    }

    pub(in crate::application) fn browse_selected_entry(&self) -> Operations {
        let Some(url) = self.selected_entry_url() else {
            return Operations::Nop;
        };
        [
            Operation::OpenTextBrowser { url },
            Operation::ForceRedrawTerminal,
        ]
        .into()
    }

    fn selected_entry_url(&self) -> Option<Url> {
        let entry_website_url = self.entries.selected_entry_website_url()?;
        Self::parse_url(entry_website_url, "entry")
    }

    fn parse_browser_url(value: &str, kind: &str) -> Option<Operation> {
        Self::parse_url(value, kind).map(|url| Operation::OpenBrowser { url })
    }

    fn parse_url(value: &str, kind: &str) -> Option<Url> {
        match Url::parse(value) {
            Ok(url) => Some(url),
            Err(error) => {
                warn!(%error, url = value, "cannot open invalid {kind} URL");
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

    /// Moves feed-backed state into its initial synchronization phase.
    pub(in crate::application) fn bootstrap(
        &mut self,
        subscriptions_first: i64,
        timeline_limit: usize,
    ) -> impl Into<Operations> {
        [
            Operation::WatchFeedEvents,
            Self::reload_subscription(subscriptions_first),
            self.begin_timeline_bootstrap(timeline_limit),
        ]
    }

    fn begin_timeline_bootstrap(&mut self, limit: usize) -> Operation {
        assert!(
            matches!(self.timeline, TimelineState::Uninitialized),
            "timeline bootstrap started more than once"
        );
        self.timeline = TimelineState::FetchingWindow { base_seq: None };
        Operation::FetchTimelineWindow { limit }
    }

    pub(in crate::application) fn refresh_timeline(&mut self) -> Option<Operation> {
        match &mut self.timeline {
            TimelineState::Uninitialized | TimelineState::FetchingWindow { .. } => None,
            TimelineState::CatchingUp { dirty, .. } => {
                *dirty = true;
                None
            }
            TimelineState::Ready { seq } => {
                let since = *seq;
                self.timeline = TimelineState::CatchingUp {
                    seq: since,
                    dirty: false,
                };
                Some(Operation::CatchUpTimeline { since })
            }
        }
    }

    pub(in crate::application) fn apply_timeline_window_chunk(
        &mut self,
        entries: Vec<payload::TimelineEntry>,
        base_seq: i64,
        limit: usize,
    ) {
        let TimelineState::FetchingWindow {
            base_seq: current_base,
        } = &mut self.timeline
        else {
            panic!("timeline window chunk received outside window bootstrap");
        };
        let populate = if let Some(current) = current_base {
            assert_eq!(
                *current, base_seq,
                "timeline window chunks used different base sequences"
            );
            Populate::Append
        } else {
            *current_base = Some(base_seq);
            Populate::Replace
        };
        self.entries.update_timeline_chunk(populate, entries, limit);
    }

    pub(in crate::application) fn complete_timeline_window(
        &mut self,
        succeeded: bool,
    ) -> Option<Operation> {
        let TimelineState::FetchingWindow { base_seq } =
            std::mem::replace(&mut self.timeline, TimelineState::Uninitialized)
        else {
            panic!("timeline window completed outside window bootstrap");
        };
        match base_seq {
            Some(seq) => {
                self.timeline = TimelineState::CatchingUp { seq, dirty: false };
                Some(Operation::CatchUpTimeline { since: seq })
            }
            None if succeeded => {
                panic!("successful timeline window completed without its first chunk")
            }
            None => None,
        }
    }

    pub(in crate::application) fn apply_timeline_changes(
        &mut self,
        changes: Vec<payload::TimelineChange>,
        seq: i64,
        limit: usize,
    ) {
        let TimelineState::CatchingUp {
            seq: current_seq, ..
        } = &mut self.timeline
        else {
            panic!("timeline changes received outside catch-up");
        };
        self.entries.apply_changes(changes, limit);
        *current_seq = seq;
    }

    pub(in crate::application) fn complete_timeline_catch_up(
        &mut self,
        succeeded: bool,
    ) -> Option<Operation> {
        let TimelineState::CatchingUp { seq, dirty } =
            std::mem::replace(&mut self.timeline, TimelineState::Uninitialized)
        else {
            panic!("timeline catch-up completed outside catch-up");
        };
        if succeeded && dirty {
            self.timeline = TimelineState::CatchingUp { seq, dirty: false };
            Some(Operation::CatchUpTimeline { since: seq })
        } else {
            self.timeline = TimelineState::Ready { seq };
            None
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

    #[cfg(feature = "integration")]
    pub(in crate::application) fn timeline_is_settled(&self) -> bool {
        matches!(
            self.timeline,
            TimelineState::Uninitialized | TimelineState::Ready { .. }
        )
    }
}
