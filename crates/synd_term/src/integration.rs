use std::io;
use tuiframework::crossterm::event::Event;

pub use crate::application::TestApplication;
use tokio::sync::mpsc::UnboundedSender;
use tokio_stream::wrappers::UnboundedReceiverStream;

#[derive(Clone)]
pub struct UnboundedSenderWrapper {
    inner: UnboundedSender<io::Result<Event>>,
}

impl UnboundedSenderWrapper {
    pub fn send(&self, event: Event) {
        self.inner.send(Ok(event)).unwrap();
    }

    pub fn send_multi<T>(&self, events: T)
    where
        T: IntoIterator<Item = Event>,
    {
        events.into_iter().for_each(|event| {
            self.send(event);
        });
    }
}

pub fn event_stream() -> (
    UnboundedSenderWrapper,
    UnboundedReceiverStream<io::Result<Event>>,
) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let tx = UnboundedSenderWrapper { inner: tx };
    let event_stream = UnboundedReceiverStream::new(rx);
    (tx, event_stream)
}

pub fn resize_event(columns: u16, rows: u16) -> Event {
    Event::Resize(columns, rows)
}

pub fn focus_gained_event() -> Event {
    Event::FocusGained
}

pub fn focus_lost_event() -> Event {
    Event::FocusLost
}
