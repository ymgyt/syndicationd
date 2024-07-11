use std::io;

use tokio::sync::mpsc::UnboundedSender;
use tokio_stream::wrappers::UnboundedReceiverStream;

pub struct UnboundedSenderWrapper {
    inner: UnboundedSender<io::Result<crossterm::event::Event>>,
}

impl UnboundedSenderWrapper {
    pub fn send(&self, event: crossterm::event::Event) {
        self.inner.send(Ok(event)).unwrap();
    }

    pub fn send_multi<T>(&self, events: T)
    where
        T: IntoIterator<Item = crossterm::event::Event>,
    {
        events.into_iter().for_each(|event| {
            self.send(event);
        });
    }
}

pub fn event_stream() -> (
    UnboundedSenderWrapper,
    UnboundedReceiverStream<io::Result<crossterm::event::Event>>,
) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let tx = UnboundedSenderWrapper { inner: tx };
    let event_stream = UnboundedReceiverStream::new(rx);
    (tx, event_stream)
}
