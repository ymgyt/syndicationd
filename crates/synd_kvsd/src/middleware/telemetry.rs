use std::fmt;

use crate::{middleware::Middleware, uow::UnitOfWork};

use synd_stdx::prelude::*;

pub(crate) struct Telemetry<MW> {
    next: MW,
}

impl<MW> Telemetry<MW> {
    pub(super) fn new(next: MW) -> Self {
        Self { next }
    }
}

impl<MW> Middleware for Telemetry<MW>
where
    MW: Middleware + Send + 'static,
    <MW as Middleware>::Error: fmt::Display,
{
    type Error = MW::Error;

    async fn handle(&mut self, uow: UnitOfWork) -> Result<(), Self::Error> {
        // TODO: emit metrics
        let result = self.next.handle(uow).await;
        match result {
            Ok(()) => info!("Handle uow"),
            // Should handle in Error mw ?
            Err(ref err) => error!("{err}"),
        }

        result
    }
}
