use futures_util::{future::BoxFuture, stream::FuturesUnordered};

use crate::command::Command;

pub(crate) type JobFuture = BoxFuture<'static, anyhow::Result<Command>>;

pub(crate) struct Jobs {
    pub futures: FuturesUnordered<JobFuture>,
}

impl Jobs {
    pub fn new() -> Self {
        Self {
            futures: FuturesUnordered::new(),
        }
    }
}
