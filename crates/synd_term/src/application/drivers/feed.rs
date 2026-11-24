use futures_util::FutureExt as _;
use synd_client::{SyndApiError, payload};
use synd_feed::types::FeedUrl;
use tracing::debug;

use crate::{
    application::{
        FeedApiRef, Generation, Populate, RequestError, RequestKind, outbound::feed::FeedApi,
    },
    event::{Event, FeedRequestEvent, FeedsEvent},
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
        context: RequestContext,
        input: payload::SubscribeFeedInput,
    ) -> (RequestKind, RequestFuture) {
        let api = self.api.clone();

        (
            RequestKind::SubscribeFeed {
                url: input.url.clone(),
            },
            async move {
                let url = input.url.clone();
                api.subscribe_feed(input)
                    .await
                    .map_err(RequestError::SyndApi)?;
                emit_feed_event(&context, FeedRequestEvent::FeedSubscribed { url });
                Ok(())
            }
            .boxed(),
        )
    }

    pub(super) fn unsubscribe_feed(
        &self,
        context: RequestContext,
        url: FeedUrl,
    ) -> (RequestKind, RequestFuture) {
        let api = self.api.clone();

        (
            RequestKind::UnsubscribeFeed { url: url.clone() },
            async move {
                api.unsubscribe_feed(url.clone())
                    .await
                    .map_err(RequestError::SyndApi)?;
                emit_feed_event(&context, FeedRequestEvent::FeedUnsubscribed { url });
                Ok(())
            }
            .boxed(),
        )
    }

    pub(super) fn fetch_subscription(
        &self,
        context: RequestContext,
        generation: Generation,
        populate: Populate,
        after: Option<String>,
        first: i64,
    ) -> (RequestKind, RequestFuture) {
        let api = self.api.clone();

        (
            RequestKind::FetchSubscription { generation },
            async move {
                let subscription = api
                    .fetch_subscription(after, Some(first))
                    .await
                    .map_err(RequestError::SyndApi)?;
                emit_feed_event(
                    &context,
                    FeedRequestEvent::SubscriptionFetched {
                        generation,
                        populate,
                        subscription,
                    },
                );
                Ok(())
            }
            .boxed(),
        )
    }

    pub(super) fn open_initial_timeline(
        &self,
        context: RequestContext,
    ) -> (RequestKind, RequestFuture) {
        let api = self.api.clone();
        (
            RequestKind::OpenInitialTimeline,
            async move {
                let timeline_id = initial_timeline(api.as_ref())
                    .await
                    .map_err(RequestError::SyndApi)?;
                emit_feed_event(
                    &context,
                    FeedRequestEvent::InitialTimelineOpened { timeline_id },
                );
                Ok(())
            }
            .boxed(),
        )
    }

    /// Fetches one bounded timeline window while keeping cursor pagination private.
    pub(super) fn fetch_timeline_window(
        &self,
        context: RequestContext,
        timeline_id: payload::TimelineId,
        limit: usize,
    ) -> (RequestKind, RequestFuture) {
        let api = self.api.clone();

        (
            RequestKind::FetchTimelineWindow { limit },
            TimelineWindowRequest::new(api, context, timeline_id, limit)
                .run()
                .boxed(),
        )
    }

    /// Fetches and coalesces all timeline changes after `since`.
    pub(super) fn catch_up_timeline(
        &self,
        context: RequestContext,
        timeline_id: payload::TimelineId,
        since: i64,
    ) -> (RequestKind, RequestFuture) {
        let api = self.api.clone();

        (
            RequestKind::CatchUpTimeline { since },
            TimelineCatchUpRequest::new(api, context, timeline_id, since)
                .run()
                .boxed(),
        )
    }
}

async fn initial_timeline(api: &dyn FeedApi) -> Result<payload::TimelineId, SyndApiError> {
    const INITIAL_NAME: &str = "all";
    if let Some(timeline) = api.list_timelines().await?.into_iter().next() {
        return Ok(timeline.id);
    }
    // create default timeline
    match api
        .create_timeline(payload::CreateTimelineInput {
            name: INITIAL_NAME.into(),
        })
        .await
    {
        Ok(created) => Ok(created.timeline_id),
        Err(error) => {
            // Another client can create the initial timeline between the list and the mutation.
            if let Ok(timelines) = api.list_timelines().await
                && let Some(timeline) = timelines
                    .into_iter()
                    .find(|timeline| timeline.name.as_str() == INITIAL_NAME)
            {
                return Ok(timeline.id);
            }
            Err(error)
        }
    }
}

fn emit_feed_event(context: &RequestContext, event: FeedRequestEvent) {
    context.emit(|request_id| Event::Feeds(FeedsEvent::Request { request_id, event }));
}

/// Owns cursor and base-sequence invariants for one bounded timeline request.
struct TimelineWindowRequest {
    api: FeedApiRef,
    context: RequestContext,
    timeline_id: payload::TimelineId,
    state: TimelineWindowState,
}

impl TimelineWindowRequest {
    fn new(
        api: FeedApiRef,
        context: RequestContext,
        timeline_id: payload::TimelineId,
        limit: usize,
    ) -> Self {
        Self {
            api,
            context,
            timeline_id,
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
        let request = payload::TimelineEntriesRequest::builder()
            .timeline_id(self.timeline_id)
            .first(self.state.page_size())
            .maybe_after(self.state.cursor())
            .build();
        debug!(
            first = request.first(),
            has_after = request.after().is_some(),
            "fetch timeline window chunk"
        );
        self.api
            .fetch_timeline_entries(request)
            .await
            .map_err(RequestError::SyndApi)
    }

    fn apply(&mut self, advance: TimelineWindowAdvance) -> Result<PageFlow, RequestError> {
        match advance {
            TimelineWindowAdvance::Complete { chunk } => {
                emit_feed_event(&self.context, chunk.into());
                Ok(PageFlow::Complete)
            }
            TimelineWindowAdvance::More { next_state, chunk } => {
                self.state = next_state;
                emit_feed_event(&self.context, chunk.into());
                Ok(PageFlow::More)
            }
            TimelineWindowAdvance::Failed { chunk, error } => {
                emit_feed_event(&self.context, chunk.into());
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
    timeline_id: payload::TimelineId,
    state: TimelineCatchUpState,
}

impl TimelineCatchUpRequest {
    fn new(
        api: FeedApiRef,
        context: RequestContext,
        timeline_id: payload::TimelineId,
        since: i64,
    ) -> Self {
        Self {
            api,
            context,
            timeline_id,
            state: TimelineCatchUpState::new(since),
        }
    }

    async fn run(mut self) -> Result<(), RequestError> {
        loop {
            let page = self.observe_next_page().await?;
            let advance = TimelineCatchUpAdvance::from(page);
            let flow = self.state.apply(advance);
            if flow.is_complete() {
                emit_feed_event(&self.context, self.state.into());
                return Ok(());
            }
        }
    }

    async fn observe_next_page(&self) -> Result<payload::TimelineChangesPayload, RequestError> {
        self.api
            .fetch_timeline_changes(
                self.timeline_id,
                self.state.since,
                TIMELINE_CHANGES_PAGE_SIZE,
            )
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

#[cfg(all(test, feature = "integration"))]
mod tests {
    use super::*;
    use crate::application::outbound::feed::{MockFeedApi, MockFeedApiResponse};

    #[tokio::test]
    async fn concurrent_first_open_recovers_the_id_of_all() {
        let all_id = payload::TimelineId::from_u128(2);
        let duplicate = serde_json::from_value(serde_json::json!({
            "message": "timeline name already exists for this subscriber: all"
        }))
        .unwrap();
        let api = MockFeedApi::new([
            MockFeedApiResponse::Timelines(Ok(vec![])),
            MockFeedApiResponse::CreateTimeline(
                "all".into(),
                Err(SyndApiError::Graphql {
                    errors: vec![duplicate],
                }),
            ),
            MockFeedApiResponse::Timelines(Ok(vec![
                payload::Timeline {
                    id: payload::TimelineId::from_u128(1),
                    name: "Tech".into(),
                },
                payload::Timeline {
                    id: all_id,
                    name: "all".into(),
                },
            ])),
        ]);
        assert_eq!(initial_timeline(&api).await.unwrap(), all_id);
    }
}
