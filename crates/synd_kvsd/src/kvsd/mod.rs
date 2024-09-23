use crate::{middleware::MiddlewareStack, uow::UowChannel};

#[expect(dead_code)]
pub struct Kvsd {
    channel: UowChannel,
    middlewares: MiddlewareStack,
}

impl Kvsd {
    pub(crate) fn new(channel: UowChannel, middlewares: MiddlewareStack) -> Self {
        Self {
            channel,
            middlewares,
        }
    }
}
