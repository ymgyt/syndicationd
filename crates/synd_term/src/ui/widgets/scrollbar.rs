use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{ScrollbarOrientation, ScrollbarState, StatefulWidget},
};

use crate::ui::Context;

pub(in crate::ui) struct Scrollbar {
    pub(in crate::ui) content_length: usize,
    pub(in crate::ui) position: usize,
}

impl Scrollbar {
    pub(in crate::ui) fn render(self, area: Rect, buf: &mut Buffer, cx: &Context<'_>) {
        let mut state = ScrollbarState::default()
            .content_length(self.content_length)
            .position(self.position);

        ratatui::widgets::Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .track_symbol(Some(" "))
            .thumb_symbol("▐")
            .style(cx.theme.base)
            .render(area, buf, &mut state);
    }
}
