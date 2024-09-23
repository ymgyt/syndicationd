mod dispatcher;
pub(crate) use dispatcher::Dispatcher;
mod stack;
pub(crate) use stack::MiddlewareStack;
mod telemetry;

use crate::uow::UnitOfWork;

pub(crate) trait Middleware {
    type Error;
    async fn handle(&mut self, uow: UnitOfWork) -> Result<(), Self::Error>;
}
