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
    command::{Command, FeedsCommand, ShellCommand, UnsubscribeCommand, UnsubscribeTarget},
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
    Uninitialized,
    FetchingWindow { base_seq: Option<i64> },
    CatchingUp { seq: i64, dirty: bool },
    Ready { seq: i64 },
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
            timeline: TimelineState::Uninitialized,
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
            self.begin_timeline_bootstrap(),
        ]
    }

    fn begin_timeline_bootstrap(&mut self) -> Operation {
        assert!(
            matches!(self.timeline, TimelineState::Uninitialized),
            "timeline bootstrap started more than once"
        );
        self.timeline = TimelineState::FetchingWindow { base_seq: None };
        Operation::FetchTimelineWindow {
            limit: self.entries_limit,
        }
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
        self.entries
            .update_timeline_chunk(populate, entries, self.entries_limit);
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

    #[cfg(feature = "integration")]
    pub(in crate::application) fn timeline_is_settled(&self) -> bool {
        matches!(
            self.timeline,
            TimelineState::Uninitialized | TimelineState::Ready { .. }
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
