use super::{EncodedEvent, EventEncodingResult};
use crate::event::{CrawlEvent, CrawlEventKind};

impl CrawlEvent {
    pub(super) fn encode(self) -> EventEncodingResult<EncodedEvent> {
        match self {}
    }
}

impl CrawlEventKind {
    pub const fn event_type(self) -> &'static str {
        match self {}
    }
}
