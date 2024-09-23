use synd_kvsd_protocol::Connection;

use crate::{authn::principal::Principal, server::IncommingConnection, uow::UowSender};

#[expect(dead_code)]
pub(super) struct Handler {
    pub(super) principal: Principal,
    pub(super) connection: IncommingConnection<Connection>,
    pub(super) sender: UowSender,
}

impl Handler {
    pub(super) fn new(connection: IncommingConnection<Connection>, sender: UowSender) -> Self {
        Self {
            principal: Principal::AnonymousUser,
            connection,
            sender,
        }
    }

    #[expect(clippy::unused_async)]
    pub(super) async fn handle(self) {
        todo!()
    }
}
