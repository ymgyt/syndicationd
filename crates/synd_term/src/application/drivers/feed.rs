use futures_util::FutureExt as _;
use synd_client::{SyndApiError, payload};
use synd_feed::types::FeedUrl;
use tracing::debug;

use crate::{
    application::{FeedApiRef, Populate, RequestError},
    event::FeedRequestEvent,
};

use super::{
    feed_event_watcher::FeedEventWatcher,
    request::{RequestContext, RequestFuture},
};

const TIMELINE_WINDOW_PAGE_SIZE: usize = 250;
const TIMELINE_CHANGES_PAGE_SIZE: i64 = 200;

/// Executes feed API requests and owns the long-lived feed event source.
pub(super) struct FeedDriver {
    api: FeedApiRef,
    pub(super) watcher: FeedEventWatcher,
}

impl FeedDriver {
    pub(super) fn new(api: FeedApiRef) -> Self {
        Self {
            api,
            watcher: FeedEventWatcher::new(),
        }
    }

    pub(super) fn watch_events(&mut self) {
        self.watcher.start(self.api.clone());
    }

    pub(super) fn restart_events_if_started(&mut self) {
        self.watcher.restart_if_started(self.api.clone());
    }

    pub(super) fn subscribe_feed(
        &self,
        input: payload::SubscribeFeedInput,
    ) -> impl FnOnce(RequestContext) -> RequestFuture + use<> {
        let api = self.api.clone();

        move |context| {
            async move {
                let url = input.url.clone();
                api.subscribe_feed(input)
                    .await
                    .map_err(RequestError::SyndApi)?;
                context.emit_feeds(FeedRequestEvent::FeedSubscribed { url });
                Ok(())
            }
            .boxed()
        }
    }

    pub(super) fn unsubscribe_feed(
        &self,
        url: FeedUrl,
    ) -> impl FnOnce(RequestContext) -> RequestFuture + use<> {
        let api = self.api.clone();

        move |context| {
            async move {
                api.unsubscribe_feed(url.clone())
                    .await
                    .map_err(RequestError::SyndApi)?;
                context.emit_feeds(FeedRequestEvent::FeedUnsubscribed { url });
                Ok(())
            }
            .boxed()
        }
    }

    pub(super) fn fetch_subscription(
        &self,
        populate: Populate,
        after: Option<String>,
        first: i64,
    ) -> impl FnOnce(RequestContext) -> RequestFuture + use<> {
        let api = self.api.clone();

        move |context| {
            async move {
                let subscription = api
                    .fetch_subscription(after, Some(first))
                    .await
                    .map_err(RequestError::SyndApi)?;
                context.emit_feeds(FeedRequestEvent::SubscriptionFetched {
                    populate,
                    subscription,
                });
                Ok(())
            }
            .boxed()
        }
    }

    /// Fetches one bounded timeline window while keeping cursor pagination private.
    pub(super) fn fetch_timeline_window(
        &self,
        limit: usize,
    ) -> impl FnOnce(RequestContext) -> RequestFuture + use<> {
        let api = self.api.clone();

        move |context| {
            TimelineWindowRequest::new(api, context, limit)
                .run()
                .boxed()
        }
    }

    /// Fetches and coalesces all timeline changes after `since`.
    pub(super) fn catch_up_timeline(
        &self,
        since: i64,
    ) -> impl FnOnce(RequestContext) -> RequestFuture + use<> {
        let api = self.api.clone();

        move |context| {
            TimelineCatchUpRequest::new(api, context, since)
                .run()
                .boxed()
        }
    }
}

/// Owns cursor and base-sequence invariants for one bounded timeline request.
struct TimelineWindowRequest {
    api: FeedApiRef,
    context: RequestContext,
    state: TimelineWindowState,
}

impl TimelineWindowRequest {
    fn new(api: FeedApiRef, context: RequestContext, limit: usize) -> Self {
        Self {
            api,
            context,
            state: TimelineWindowState::new(limit),
        }
    }

    async fn run(mut self) -> Result<(), RequestError> {
        loop {
            let page = self.observe_next_page().await?;
            let advance = self.state.advance(page);
            let flow = self.apply(advance)?;
            if flow.is_complete() {
                return Ok(());
            }
        }
    }

    async fn observe_next_page(&self) -> Result<payload::TimelineEntryConnection, RequestError> {
        let first = self.state.page_size();
        debug!(
            first,
            has_after = self.state.has_cursor(),
            "fetch timeline window chunk"
        );
        self.api
            .fetch_timeline_entries(self.state.cursor(), first)
            .await
            .map_err(RequestError::SyndApi)
    }

    fn apply(&mut self, advance: TimelineWindowAdvance) -> Result<PageFlow, RequestError> {
        match advance {
            TimelineWindowAdvance::Complete { chunk } => {
                self.context.emit_feeds(chunk.into());
                Ok(PageFlow::Complete)
            }
            TimelineWindowAdvance::More { next_state, chunk } => {
                self.state = next_state;
                self.context.emit_feeds(chunk.into());
                Ok(PageFlow::More)
            }
            TimelineWindowAdvance::Failed { chunk, error } => {
                self.context.emit_feeds(chunk.into());
                Err(RequestError::SyndApi(error))
            }
        }
    }
}

/// Transport-pagination state for one timeline window request.
struct TimelineWindowState {
    after: Option<String>,
    remaining: usize,
    base_seq: Option<i64>,
}

impl TimelineWindowState {
    fn new(limit: usize) -> Self {
        Self {
            after: None,
            remaining: limit,
            base_seq: None,
        }
    }

    fn page_size(&self) -> i64 {
        i64::try_from(self.remaining.min(TIMELINE_WINDOW_PAGE_SIZE))
            .expect("timeline page size fits in i64")
    }

    fn has_cursor(&self) -> bool {
        self.after.is_some()
    }

    fn cursor(&self) -> Option<String> {
        self.after.clone()
    }

    fn advance(&self, page: payload::TimelineEntryConnection) -> TimelineWindowAdvance {
        let base_seq = self.base_seq.unwrap_or(page.seq);
        let mut entries = page.nodes;
        entries.truncate(self.remaining);
        let remaining = self.remaining.saturating_sub(entries.len());
        let chunk = TimelineWindowChunk { entries, base_seq };
        match self.next_page(remaining, page.page_info) {
            Ok(TimelineWindowContinuation::Complete) => TimelineWindowAdvance::Complete { chunk },
            Ok(TimelineWindowContinuation::More { after }) => TimelineWindowAdvance::More {
                next_state: Self {
                    after: Some(after),
                    remaining,
                    base_seq: Some(base_seq),
                },
                chunk,
            },
            Err(error) => TimelineWindowAdvance::Failed { chunk, error },
        }
    }

    fn next_page(
        &self,
        remaining: usize,
        page_info: payload::PageInfo,
    ) -> Result<TimelineWindowContinuation, SyndApiError> {
        if remaining == 0 {
            return Ok(TimelineWindowContinuation::Complete);
        }

        let payload::PageInfo::More { next_cursor } = page_info else {
            return Ok(TimelineWindowContinuation::Complete);
        };
        if self.after.as_ref() == Some(&next_cursor) {
            return Err(SyndApiError::UnexpectedResponse {
                context: "timeline pagination cursor did not advance",
            });
        }
        Ok(TimelineWindowContinuation::More { after: next_cursor })
    }
}

/// Valid pagination outcome after one timeline window page.
enum TimelineWindowContinuation {
    Complete,
    More { after: String },
}

/// State change or protocol failure derived from one timeline window page.
enum TimelineWindowAdvance {
    Complete {
        chunk: TimelineWindowChunk,
    },
    More {
        next_state: TimelineWindowState,
        chunk: TimelineWindowChunk,
    },
    Failed {
        chunk: TimelineWindowChunk,
        error: SyndApiError,
    },
}

/// Application-visible portion of one validated timeline window page.
struct TimelineWindowChunk {
    entries: Vec<payload::TimelineEntry>,
    base_seq: i64,
}

impl From<TimelineWindowChunk> for FeedRequestEvent {
    fn from(chunk: TimelineWindowChunk) -> Self {
        Self::TimelineWindowChunkFetched {
            entries: chunk.entries,
            base_seq: chunk.base_seq,
        }
    }
}

/// Owns page coalescing for one logical timeline catch-up request.
struct TimelineCatchUpRequest {
    api: FeedApiRef,
    context: RequestContext,
    state: TimelineCatchUpState,
}

impl TimelineCatchUpRequest {
    fn new(api: FeedApiRef, context: RequestContext, since: i64) -> Self {
        Self {
            api,
            context,
            state: TimelineCatchUpState::new(since),
        }
    }

    async fn run(mut self) -> Result<(), RequestError> {
        loop {
            let page = self.observe_next_page().await?;
            let advance = TimelineCatchUpAdvance::from(page);
            let flow = self.state.apply(advance);
            if flow.is_complete() {
                self.context.emit_feeds(self.state.into());
                return Ok(());
            }
        }
    }

    async fn observe_next_page(&self) -> Result<payload::TimelineChangesPayload, RequestError> {
        self.api
            .fetch_timeline_changes(self.state.since, TIMELINE_CHANGES_PAGE_SIZE)
            .await
            .map_err(RequestError::SyndApi)
    }
}

/// Accumulated result of one logical timeline catch-up request.
struct TimelineCatchUpState {
    since: i64,
    changes: Vec<payload::TimelineChange>,
}

impl TimelineCatchUpState {
    fn new(since: i64) -> Self {
        Self {
            since,
            changes: Vec::new(),
        }
    }

    fn apply(&mut self, advance: TimelineCatchUpAdvance) -> PageFlow {
        self.changes.extend(advance.changes);
        self.since = advance.seq;
        advance.flow
    }
}

impl From<TimelineCatchUpState> for FeedRequestEvent {
    fn from(state: TimelineCatchUpState) -> Self {
        Self::TimelineChangesFetched {
            changes: state.changes,
            seq: state.since,
        }
    }
}

/// Pure state change described by one observed timeline changes page.
struct TimelineCatchUpAdvance {
    changes: Vec<payload::TimelineChange>,
    seq: i64,
    flow: PageFlow,
}

impl From<payload::TimelineChangesPayload> for TimelineCatchUpAdvance {
    fn from(page: payload::TimelineChangesPayload) -> Self {
        Self {
            changes: page.changes,
            seq: page.seq,
            flow: PageFlow::from(page.has_more),
        }
    }
}

#[derive(Clone, Copy)]
enum PageFlow {
    More,
    Complete,
}

impl From<bool> for PageFlow {
    fn from(has_more: bool) -> Self {
        if has_more { Self::More } else { Self::Complete }
    }
}

impl PageFlow {
    fn is_complete(self) -> bool {
        matches!(self, Self::Complete)
    }
}
