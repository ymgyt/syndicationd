use futures_util::future::BoxFuture;
use tokio::sync::mpsc;

use crate::{
    application::{RequestError, RequestId},
    event::Event,
};

pub(super) type RequestFuture = BoxFuture<'static, Result<(), RequestError>>;
pub(super) type JobFuture = BoxFuture<'static, ()>;

/// Emits domain events correlated to one registered external request.
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

    pub(super) fn emit(&self, make_event: impl FnOnce(RequestId) -> Event) {
        self.event_tx
            .send(make_event(self.request_id))
            .expect("Drivers owns the event receiver");
    }
}
