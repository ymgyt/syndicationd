use indexmap::IndexMap;

use super::{RequestId, RequestKind};

/// Requests shown on the status line in request emission order.
pub(crate) struct InFlightRequests {
    requests: IndexMap<RequestId, InFlightRequest>,
    throbber_step: i8,
}

/// Application-visible state of one registered request.
struct InFlightRequest {
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

    pub(super) fn register(&mut self, request_id: RequestId, kind: RequestKind) {
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

    pub(super) fn advance_timeline_window(&mut self, request_id: RequestId, loaded: usize) {
        self.requests
            .get_mut(&request_id)
            .expect("driver emitted progress for an unknown request")
            .add_timeline_window_chunk(loaded);
    }

    pub(super) fn complete(&mut self, request_id: RequestId) -> RequestKind {
        self.requests
            .shift_remove(&request_id)
            .expect("driver completed an unknown request")
            .kind
    }

    pub(crate) fn tick(&mut self) {
        self.throbber_step = self.throbber_step.wrapping_add(1);
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

    pub(crate) fn is_empty(&self) -> bool {
        self.requests.is_empty()
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
