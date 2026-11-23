use std::rc::Rc;

use itertools::Itertools;
use ratatui::buffer::Buffer;
use synd_client::payload;
use synd_feed::types::FeedUrl;
use tracing::{debug, warn};
use url::Url;

use super::{Component, RenderContext};
use crate::{
    application::{
        Config, Generation, Populate, RequestError, RequestKind, effects::Effects,
        input_parser::InputParser, request::synd_api_error_message,
    },
    command::{
        Command, FeedsCommand, ReaderCommand, ShellCommand, UnsubscribeCommand, UnsubscribeTarget,
    },
    config::Categories,
    event::{Event, EventKind, FeedRequestEvent, FeedsEvent},
    keymap::Layer,
    operation::{Operation, Operations},
    types::EntryExt,
    ui::widgets::{entries::EntriesWidget, subscription::SubscriptionWidget, tabs::Tab},
};

/// Timeline bootstrap and catch-up state.
#[derive(Debug)]
pub(crate) enum TimelineState {
    WaitingForApi,
    Opening,
    LoadFailed {
        timeline_id: Option<payload::TimelineId>,
    },
    FetchingWindow {
        timeline_id: payload::TimelineId,
        base_seq: Option<i64>,
    },
    CatchingUp {
        timeline_id: payload::TimelineId,
        seq: i64,
        dirty: bool,
    },
    Ready {
        timeline_id: payload::TimelineId,
        seq: i64,
    },
}

/// Feed subscription and timeline application state.
pub(crate) struct FeedsComponent {
    pub(crate) subscription: SubscriptionWidget,
    matcher: crate::matcher::Matcher,
    pub(crate) entries: EntriesWidget,
    timeline: TimelineState,
    subscription_generation: Generation,
    page_size: i64,
    entries_limit: usize,
    categories: Rc<Categories>,
}

impl FeedsComponent {
    pub(in crate::application) fn apply_command(&mut self, command: FeedsCommand) -> Effects {
        let operations = match command {
            FeedsCommand::PromptFeedSubscription => Operation::OpenFeedSubscriptionEditor.into(),
            FeedsCommand::PromptFeedEdition => self.edit_selected_feed().into(),
            FeedsCommand::PromptFeedUnsubscription => {
                let Some(feed) = self.subscription.selected_feed() else {
                    return Effects::NONE;
                };
                return Effects::NONE.with_command(Command::UnsubscribePopup(
                    UnsubscribeCommand::Open(UnsubscribeTarget {
                        url: feed.url.clone(),
                        title: feed.title.clone(),
                    }),
                ));
            }
            FeedsCommand::ReloadSubscription => self.reload_subscription().into(),
            FeedsCommand::OpenFeed => self.open_selected_feed().into(),
            FeedsCommand::RefreshTimeline => self.refresh_timeline().into(),
            FeedsCommand::ReadEntry => return self.read_selected_entry(),
            FeedsCommand::OpenEntry => self.open_selected_entry().into(),
            FeedsCommand::BrowseEntry => self.browse_selected_entry(),
        };
        Effects::operations(operations)
    }

    pub(super) fn new(config: &Config, categories: Rc<Categories>) -> Self {
        Self {
            subscription_generation: Generation::default(),
            page_size: config.feeds_per_pagination,
            entries_limit: config.entries_limit,
            categories,
            subscription: SubscriptionWidget::new(),
            matcher: crate::matcher::Matcher::new(),
            entries: EntriesWidget::new(),
            timeline: TimelineState::WaitingForApi,
        }
    }

    pub(in crate::application) fn is_already_subscribed(&self, url: &FeedUrl) -> bool {
        self.subscription.is_already_subscribed(url)
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
        Self::entry_url(self.entries.selected_entry()?).map(|url| Operation::OpenBrowser { url })
    }

    fn read_selected_entry(&self) -> Effects {
        let Some(entry) = self.entries.selected_entry() else {
            return Effects::NONE;
        };
        match Self::entry_url(entry) {
            Some(url) => Command::Reader(ReaderCommand::Open(url)),
            None => Command::Shell(ShellCommand::ShowError(
                "This entry has no article URL".into(),
            )),
        }
        .into()
    }

    pub(in crate::application) fn browse_selected_entry(&self) -> Operations {
        let Some(url) = self.entries.selected_entry().and_then(Self::entry_url) else {
            return Operations::Nop;
        };
        [
            Operation::OpenTextBrowser { url },
            Operation::ForceRedrawTerminal,
        ]
        .into()
    }

    fn entry_url(entry: &payload::Entry) -> Option<Url> {
        Self::parse_url(entry.website_url.as_deref()?, "entry")
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

    pub(in crate::application) fn reload_subscription(&mut self) -> Operation {
        self.subscription_generation.advance();
        Operation::FetchSubscription {
            generation: self.subscription_generation,
            populate: Populate::Replace,
            after: None,
            first: self.page_size,
        }
    }

    /// Moves feed-backed state into its initial synchronization phase.
    fn bootstrap(&mut self) -> impl Into<Operations> {
        [
            Operation::WatchFeedEvents,
            self.reload_subscription(),
            self.open_initial_timeline(),
        ]
    }

    fn open_initial_timeline(&mut self) -> Operation {
        self.timeline = TimelineState::Opening;
        Operation::OpenInitialTimeline
    }

    fn fetch_timeline_window(&mut self, timeline_id: payload::TimelineId) -> Operation {
        self.timeline = TimelineState::FetchingWindow {
            timeline_id,
            base_seq: None,
        };
        Operation::FetchTimelineWindow {
            timeline_id,
            limit: self.entries_limit,
        }
    }

    pub(in crate::application) fn refresh_timeline(&mut self) -> Option<Operation> {
        match self.timeline {
            TimelineState::WaitingForApi
            | TimelineState::Opening
            | TimelineState::FetchingWindow { .. } => None,
            TimelineState::LoadFailed { timeline_id: None } => Some(self.open_initial_timeline()),
            TimelineState::LoadFailed {
                timeline_id: Some(id),
            } => Some(self.fetch_timeline_window(id)),
            TimelineState::CatchingUp { ref mut dirty, .. } => {
                *dirty = true;
                None
            }
            TimelineState::Ready { timeline_id, seq } => {
                Some(self.catch_up_timeline(timeline_id, seq))
            }
        }
    }

    pub(in crate::application) fn apply_timeline_window_chunk(
        &mut self,
        entries: Vec<payload::TimelineEntry>,
        base_seq: i64,
    ) {
        let TimelineState::FetchingWindow {
            base_seq: current_base,
            ..
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
        self.entries
            .update_timeline_chunk(populate, entries, self.entries_limit);
    }

    pub(in crate::application) fn complete_timeline_window(
        &mut self,
        succeeded: bool,
    ) -> Option<Operation> {
        let TimelineState::FetchingWindow {
            timeline_id,
            base_seq,
        } = self.timeline
        else {
            panic!("timeline window completed outside window bootstrap");
        };
        if !succeeded {
            self.timeline = TimelineState::LoadFailed {
                timeline_id: Some(timeline_id),
            };
            return None;
        }
        let seq = base_seq.expect("successful timeline window completed without its first chunk");
        Some(self.catch_up_timeline(timeline_id, seq))
    }

    fn catch_up_timeline(&mut self, timeline_id: payload::TimelineId, since: i64) -> Operation {
        self.timeline = TimelineState::CatchingUp {
            timeline_id,
            seq: since,
            dirty: false,
        };
        Operation::CatchUpTimeline { timeline_id, since }
    }

    pub(in crate::application) fn apply_timeline_changes(
        &mut self,
        changes: Vec<payload::TimelineChange>,
        seq: i64,
    ) {
        let TimelineState::CatchingUp {
            seq: current_seq, ..
        } = &mut self.timeline
        else {
            panic!("timeline changes received outside catch-up");
        };
        self.entries.apply_changes(changes, self.entries_limit);
        *current_seq = seq;
    }

    pub(in crate::application) fn complete_timeline_catch_up(
        &mut self,
        succeeded: bool,
    ) -> Option<Operation> {
        let TimelineState::CatchingUp {
            timeline_id,
            seq,
            dirty,
        } = self.timeline
        else {
            panic!("timeline catch-up completed outside catch-up");
        };
        if succeeded && dirty {
            Some(self.catch_up_timeline(timeline_id, seq))
        } else {
            self.timeline = TimelineState::Ready { timeline_id, seq };
            None
        }
    }

    #[cfg(feature = "integration")]
    pub(in crate::application) fn timeline_is_settled(&self) -> bool {
        matches!(
            self.timeline,
            TimelineState::WaitingForApi
                | TimelineState::LoadFailed { .. }
                | TimelineState::Ready { .. }
        )
    }
}

impl Component for FeedsComponent {
    fn render(&self, buf: &mut Buffer, cx: &RenderContext<'_>) {
        self.subscription.render(cx.frame.content, buf, &cx.ui);
    }

    fn key_layers(&self) -> &[Layer] {
        &[Layer::List, Layer::Feeds]
    }
    fn subscriptions(&self) -> &'static [EventKind] {
        &[
            EventKind::ApiAccessReady,
            EventKind::ApiAccessRefreshed,
            EventKind::TabChanged,
            EventKind::FeedFilterChanged,
            EventKind::Feeds,
            EventKind::RequestCompleted,
            EventKind::FeedSubscriptionEditorClosed,
            EventKind::FeedEditionEditorClosed,
        ]
    }

    fn apply_event(&mut self, event: &Event) -> Effects {
        match event {
            Event::ApiAccessReady => Effects::operations(self.bootstrap()),
            Event::ApiAccessRefreshed => Effects::operations(self.refresh_timeline()),
            Event::TabChanged(Tab::Feeds) if !self.subscription.has_subscription() => {
                Effects::operations(self.reload_subscription())
            }
            Event::FeedFilterChanged {
                requirement,
                criteria,
            } => {
                let filter = criteria.feed_filterer(*requirement, &mut self.matcher);
                self.entries.update_filterer(filter.clone());
                self.subscription.update_filterer(filter);
                Effects::NONE
            }
            Event::Feeds(FeedsEvent::Request { event, .. }) => self.apply_feed_request(event),
            Event::Feeds(FeedsEvent::Push {
                event: payload::FeedEvent::TimelineChanged(event),
            }) => {
                debug!(changed_at = %event.changed_at, affected_feeds = ?event.affected_feeds, "timeline changed");
                Effects::operations(self.refresh_timeline())
            }
            Event::RequestCompleted { kind, result, .. } => {
                let follow_up = match kind {
                    RequestKind::FetchSubscription { generation }
                        if *generation != self.subscription_generation =>
                    {
                        return Effects::NONE;
                    }
                    RequestKind::OpenInitialTimeline if result.is_err() => {
                        self.timeline = TimelineState::LoadFailed { timeline_id: None };
                        None
                    }
                    RequestKind::FetchTimelineWindow { .. } => {
                        self.complete_timeline_window(result.is_ok())
                    }
                    RequestKind::CatchUpTimeline { .. } => {
                        self.complete_timeline_catch_up(result.is_ok())
                    }
                    _ => None,
                };
                let effects = Effects::operations(follow_up);
                if let Err(RequestError::SyndApi(error)) = result {
                    effects.with_command(Command::show_request_error(
                        kind,
                        synd_api_error_message(error),
                    ))
                } else {
                    effects
                }
            }
            Event::FeedSubscriptionEditorClosed { input }
            | Event::FeedEditionEditorClosed { input } => {
                match InputParser::new(input).parse_feed_subscription(&self.categories) {
                    Ok(input)
                        if matches!(event, Event::FeedSubscriptionEditorClosed { .. })
                            && self.is_already_subscribed(&input.url) =>
                    {
                        Command::Shell(ShellCommand::ShowError(format!(
                            "{} already subscribed",
                            input.url
                        )))
                        .into()
                    }
                    Ok(input) => Effects::operations(Operation::SubscribeFeed { input }),
                    Err(error) => Command::Shell(ShellCommand::ShowError(error.to_string())).into(),
                }
            }
            _ => Effects::NONE,
        }
    }
}

impl FeedsComponent {
    fn apply_feed_request(&mut self, event: &FeedRequestEvent) -> Effects {
        match event {
            FeedRequestEvent::InitialTimelineOpened { timeline_id } => {
                return Effects::operations(self.fetch_timeline_window(*timeline_id));
            }
            FeedRequestEvent::FeedSubscribed { url } => {
                debug!(%url, "feed subscribed");
                return Effects::operations(self.reload_subscription());
            }
            FeedRequestEvent::SubscriptionFetched {
                generation,
                populate,
                subscription,
            } => {
                if *generation != self.subscription_generation {
                    return Effects::NONE;
                }
                let next = match &subscription.feeds.page_info {
                    payload::PageInfo::Complete { .. } => None,
                    payload::PageInfo::More { next_cursor } => Some(Operation::FetchSubscription {
                        generation: *generation,
                        populate: Populate::Append,
                        after: Some(next_cursor.clone()),
                        first: subscription.feeds.nodes.len().try_into().unwrap_or(0),
                    }),
                };
                self.subscription
                    .update_subscription(*populate, subscription.clone());
                return Effects::operations(next);
            }
            FeedRequestEvent::FeedUnsubscribed { url } => self.feed_unsubscribed(url),
            FeedRequestEvent::TimelineWindowChunkFetched { entries, base_seq } => {
                self.apply_timeline_window_chunk(entries.clone(), *base_seq);
            }
            FeedRequestEvent::TimelineChangesFetched { changes, seq } => {
                self.apply_timeline_changes(changes.clone(), *seq);
            }
        }
        Event::FeedCategoriesChanged(
            self.entries
                .entries()
                .map(EntryExt::category)
                .unique()
                .cloned()
                .collect(),
        )
        .into()
    }
}
