use std::io;

use crate::terminal::Terminal;
use ratatui::backend::TestBackend;
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

pub fn new_test_terminal(width: u16, height: u16) -> Terminal {
    let backend = TestBackend::new(width, height);
    let terminal = ratatui::Terminal::new(backend).unwrap();
    Terminal::with(terminal)
}

pub fn resize_event(columns: u16, rows: u16) -> crossterm::event::Event {
    crossterm::event::Event::Resize(columns, rows)
}

pub fn focus_gained_event() -> crossterm::event::Event {
    crossterm::event::Event::FocusGained
}

pub fn focus_lost_event() -> crossterm::event::Event {
    crossterm::event::Event::FocusLost
}
