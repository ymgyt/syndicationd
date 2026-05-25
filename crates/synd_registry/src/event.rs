use tokio::sync::broadcast;

use crate::model::RegistryEvent;

#[derive(Clone)]
pub struct RegistryEventPublisher {
    sender: broadcast::Sender<RegistryEvent>,
}

pub struct RegistryEventSubscriber {
    receiver: broadcast::Receiver<RegistryEvent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryEventRecvError {
    Closed,
    Lagged(u64),
}

impl RegistryEventPublisher {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn subscribe(&self) -> RegistryEventSubscriber {
        RegistryEventSubscriber {
            receiver: self.sender.subscribe(),
        }
    }

    pub fn publish(&self, event: RegistryEvent) -> usize {
        self.sender.send(event).unwrap_or_default()
    }
}

impl Default for RegistryEventPublisher {
    fn default() -> Self {
        Self::new(1024)
    }
}

impl RegistryEventSubscriber {
    pub async fn recv(&mut self) -> Result<RegistryEvent, RegistryEventRecvError> {
        self.receiver.recv().await.map_err(|err| match err {
            broadcast::error::RecvError::Closed => RegistryEventRecvError::Closed,
            broadcast::error::RecvError::Lagged(skipped) => RegistryEventRecvError::Lagged(skipped),
        })
    }
}
