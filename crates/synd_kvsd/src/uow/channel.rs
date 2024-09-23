use tokio::sync::mpsc;

use crate::uow::UnitOfWork;

#[derive(Clone)]
pub struct UowSender {
    tx: mpsc::Sender<UnitOfWork>,
}

pub(crate) struct UowReceiver {
    rx: mpsc::Receiver<UnitOfWork>,
}

impl UowReceiver {
    pub(crate) async fn recv(&mut self) -> Option<UnitOfWork> {
        self.rx.recv().await
    }
}

pub(crate) struct UowChannel {
    tx: UowSender,
    rx: UowReceiver,
}

impl UowChannel {
    pub(crate) fn new(buffer: usize) -> Self {
        let (tx, rx) = mpsc::channel(buffer);
        Self {
            tx: UowSender { tx },
            rx: UowReceiver { rx },
        }
    }

    pub(crate) fn split(self) -> (UowSender, UowReceiver) {
        (self.tx, self.rx)
    }
}
