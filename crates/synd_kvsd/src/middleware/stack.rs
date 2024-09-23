use crate::{
    middleware::{telemetry::Telemetry, Dispatcher, Middleware},
    uow::UnitOfWork,
};

pub(crate) struct MiddlewareStack {
    root: Telemetry<Dispatcher>,
}

impl MiddlewareStack {
    pub(crate) fn new(dispatcher: Dispatcher) -> Self {
        let telemetry = Telemetry::new(dispatcher);

        Self { root: telemetry }
    }

    #[expect(dead_code)]
    pub(crate) async fn handle(&mut self, uow: UnitOfWork) {
        self.root.handle(uow).await.unwrap();
    }
}
