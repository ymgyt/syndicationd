use ratatui::prelude::{Constraint, Direction, Layout, Rect};

pub(super) trait RectExt {
    /// Create centered Rect
    fn centered(self, percent_x: u16, percent_y: u16) -> Rect;
}

impl RectExt for Rect {
    fn centered(self, percent_x: u16, percent_y: u16) -> Rect {
        // get vertically centered rect
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ])
            .split(self);

        // then centered horizontally
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(layout[1])[1]
    }
}
