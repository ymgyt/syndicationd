use tokio::sync::mpsc;

use crate::crawl::scheduler::dispatch::DispatchEntry;

pub(crate) fn dispatch_queue(capacity: usize) -> (DispatchQueueWriter, DispatchQueueReader) {
    let (sender, receiver) = mpsc::channel(capacity);
    (
        DispatchQueueWriter { sender },
        DispatchQueueReader { receiver },
    )
}

/// Sender side of the worker-facing dispatch queue.
#[derive(Debug, Clone)]
pub(crate) struct DispatchQueueWriter {
    sender: mpsc::Sender<DispatchEntry>,
}

impl DispatchQueueWriter {
    pub(crate) fn len(&self) -> usize {
        self.sender
            .max_capacity()
            .saturating_sub(self.sender.capacity())
    }

    pub(crate) fn remaining_capacity(&self) -> usize {
        self.sender.capacity()
    }

    pub(crate) fn push(&self, entry: DispatchEntry) -> Result<(), DispatchQueuePushError> {
        self.sender.try_send(entry).map_err(|err| match err {
            mpsc::error::TrySendError::Full(entry) => DispatchQueuePushError::Full(entry),
            mpsc::error::TrySendError::Closed(entry) => DispatchQueuePushError::Closed(entry),
        })
    }
}

/// Receiver side of the worker-facing dispatch queue.
#[derive(Debug)]
pub(crate) struct DispatchQueueReader {
    receiver: mpsc::Receiver<DispatchEntry>,
}

impl DispatchQueueReader {
    pub(crate) async fn pop(&mut self) -> Option<DispatchEntry> {
        self.receiver.recv().await
    }
}

#[derive(Debug)]
pub(crate) enum DispatchQueuePushError {
    Full(DispatchEntry),
    Closed(DispatchEntry),
}
