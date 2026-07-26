use indexmap::IndexMap;

use crate::event::{AuthEvent, FeedRequestEvent, GhEvent};

use super::{RequestId, RequestKind};

/// Application-owned request display state in request emission order.
pub(crate) struct InFlightRequests {
    requests: IndexMap<RequestId, InFlightRequest>,
    throbber_step: i8,
}

/// Application-visible state of one registered request.
pub(crate) struct InFlightRequest {
    kind: RequestKind,
    progress: Option<RequestProgress>,
}

/// Determinate progress available for selected request kinds.
pub(crate) enum RequestProgress {
    TimelineWindow { loaded: usize, target: usize },
}

/// The request selected for the global status line.
pub(crate) struct InFlightStatus<'a> {
    request: &'a InFlightRequest,
    throbber_step: i8,
    other_count: usize,
}

impl InFlightRequests {
    pub(crate) fn new() -> Self {
        Self {
            requests: IndexMap::new(),
            throbber_step: 0,
        }
    }

    pub(crate) fn register(&mut self, request_id: RequestId, kind: RequestKind) {
        let progress = match &kind {
            RequestKind::FetchTimelineWindow { limit } => Some(RequestProgress::TimelineWindow {
                loaded: 0,
                target: *limit,
            }),
            _ => None,
        };
        let previous = self
            .requests
            .insert(request_id, InFlightRequest { kind, progress });
        assert!(previous.is_none(), "driver emitted a duplicate request id");
        if self.requests.len() == 1 {
            self.throbber_step = 0;
        }
    }

    pub(crate) fn correlate_auth_event(&self, request_id: RequestId, event: &AuthEvent) {
        match (self.kind(request_id), event) {
            (
                RequestKind::StartDeviceFlow { provider: expected },
                AuthEvent::DeviceFlowAuthorizationReceived { provider, .. },
            ) => assert_eq!(
                expected, provider,
                "device authorization provider did not match its request"
            ),
            (
                RequestKind::PollDeviceFlowAccessToken { .. },
                AuthEvent::DeviceFlowCredentialReceived { .. },
            ) => {}
            _ => panic!("authentication event did not match its request"),
        }
    }

    pub(crate) fn correlate_feed_event(&mut self, request_id: RequestId, event: &FeedRequestEvent) {
        let request = self
            .requests
            .get_mut(&request_id)
            .expect("driver emitted a request event for an unknown request");
        match (&request.kind, event) {
            (
                RequestKind::SubscribeFeed { url: expected },
                FeedRequestEvent::FeedSubscribed { url },
            ) => assert_eq!(expected, url, "subscribed feed did not match its request"),
            (
                RequestKind::UnsubscribeFeed { url: expected },
                FeedRequestEvent::FeedUnsubscribed { url },
            ) => assert_eq!(expected, url, "unsubscribed feed did not match its request"),
            (RequestKind::FetchSubscription, FeedRequestEvent::SubscriptionFetched { .. })
            | (
                RequestKind::CatchUpTimeline { .. },
                FeedRequestEvent::TimelineChangesFetched { .. },
            ) => {}
            (
                RequestKind::FetchTimelineWindow { .. },
                FeedRequestEvent::TimelineWindowChunkFetched { entries, .. },
            ) => request.add_timeline_window_chunk(entries.len()),
            _ => panic!("feed event did not match its request"),
        }
    }

    pub(crate) fn correlate_gh_event(&self, request_id: RequestId, event: &GhEvent) {
        match (self.kind(request_id), event) {
            (RequestKind::FetchGhNotifications { .. }, GhEvent::NotificationsFetched { .. })
            | (RequestKind::FetchGhIssue { .. }, GhEvent::IssueFetched { .. })
            | (RequestKind::FetchGhPullRequest { .. }, GhEvent::PullRequestFetched { .. }) => {}
            (
                RequestKind::MarkGhNotificationAsDone { id: expected },
                GhEvent::NotificationMarkedAsDone { notification_id },
            ) => assert_eq!(
                expected, notification_id,
                "completed GitHub notification did not match its request"
            ),
            _ => panic!("GitHub event did not match its request"),
        }
    }

    pub(crate) fn complete(&mut self, request_id: RequestId) -> InFlightRequest {
        let request = self
            .requests
            .shift_remove(&request_id)
            .expect("driver completed an unknown request");
        if self.requests.is_empty() {
            self.throbber_step = 0;
        }
        request
    }

    pub(crate) fn tick(&mut self) {
        if !self.requests.is_empty() {
            self.throbber_step = self.throbber_step.wrapping_add(1);
        }
    }

    pub(crate) fn status(&self) -> Option<InFlightStatus<'_>> {
        let request = self
            .requests
            .values()
            .rev()
            .find(|request| request.progress.is_some())
            .or_else(|| self.requests.last().map(|(_, request)| request))?;
        Some(InFlightStatus {
            request,
            throbber_step: self.throbber_step,
            other_count: self.requests.len() - 1,
        })
    }

    #[cfg(feature = "integration")]
    pub(crate) fn is_empty(&self) -> bool {
        self.requests.is_empty()
    }

    fn kind(&self, request_id: RequestId) -> &RequestKind {
        &self
            .requests
            .get(&request_id)
            .expect("driver emitted a request event for an unknown request")
            .kind
    }
}

impl InFlightRequest {
    fn add_timeline_window_chunk(&mut self, loaded: usize) {
        let Some(RequestProgress::TimelineWindow {
            loaded: current,
            target,
        }) = self.progress.as_mut()
        else {
            panic!("driver emitted a timeline chunk for a non-window request");
        };
        *current = current.saturating_add(loaded).min(*target);
    }

    pub(crate) fn into_kind(self) -> RequestKind {
        self.kind
    }
}

impl InFlightStatus<'_> {
    pub(crate) fn kind(&self) -> &RequestKind {
        &self.request.kind
    }

    pub(crate) fn progress(&self) -> Option<&RequestProgress> {
        self.request.progress.as_ref()
    }

    pub(crate) fn throbber_step(&self) -> i8 {
        self.throbber_step
    }

    pub(crate) fn other_count(&self) -> usize {
        self.other_count
    }
}
