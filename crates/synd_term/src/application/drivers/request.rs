use futures_util::future::BoxFuture;
use tokio::sync::mpsc;

use crate::{
    application::{RequestError, RequestId},
    event::{AuthEvent, Event, FeedRequestEvent, FeedsEvent, GhEvent},
};

pub(super) type RequestFuture = BoxFuture<'static, Result<(), RequestError>>;
pub(super) type JobFuture = BoxFuture<'static, ()>;

/// Emits domain events correlated to one registered external request.
#[derive(Clone)]
pub(super) struct RequestContext {
    request_id: RequestId,
    event_tx: mpsc::UnboundedSender<Event>,
}

impl RequestContext {
    pub(super) fn new(request_id: RequestId, event_tx: mpsc::UnboundedSender<Event>) -> Self {
        Self {
            request_id,
            event_tx,
        }
    }

    pub(super) fn emit_auth(&self, event: AuthEvent) {
        self.emit(Event::Auth {
            request_id: self.request_id,
            event,
        });
    }

    pub(super) fn emit_feeds(&self, event: FeedRequestEvent) {
        self.emit(Event::Feeds(FeedsEvent::Request {
            request_id: self.request_id,
            event,
        }));
    }

    pub(super) fn emit_gh(&self, event: GhEvent) {
        self.emit(Event::Gh {
            request_id: self.request_id,
            event,
        });
    }

    fn emit(&self, event: Event) {
        self.event_tx
            .send(event)
            .expect("Drivers owns the event receiver");
    }
}
