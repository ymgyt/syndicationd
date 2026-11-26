use super::component::Ui;
use crate::event::Event;

pub(super) type Effects = tuiframework::Effects<Ui>;

impl From<Event> for Effects {
    fn from(event: Event) -> Self {
        Self::NONE.emit(event)
    }
}
