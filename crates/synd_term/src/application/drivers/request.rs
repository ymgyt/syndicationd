use crate::{application::RequestError, event::Event};
use futures_util::future::BoxFuture;
pub(super) type RequestFuture = BoxFuture<'static, Result<(), RequestError>>;
pub(super) type RequestContext = tuiframework::driver::RequestContext<Event>;
