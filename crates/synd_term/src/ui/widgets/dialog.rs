use crate::ui::theme::Theme;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    text::Span,
    widgets::{Block, Clear, Padding, Widget},
};

/// A centered popup with a dimmed backdrop, a body, and an action footer.
pub(crate) struct Dialog {
    backdrop: Rect,
    available: Rect,
    width: u16,
    padding_x: u16,
    padding_y: u16,
}

impl Dialog {
    pub(crate) fn new(backdrop: Rect, available: Rect) -> Self {
        let width = available.width.min(58);
        let padding_x = if width < 48 { 1 } else { 2 };
        let padding_y = u16::from(available.height >= 14);
        Self {
            backdrop,
            available,
            width,
            padding_x,
            padding_y,
        }
    }

    pub(crate) fn inner_width(&self) -> u16 {
        self.width.saturating_sub(2 + 2 * self.padding_x)
    }

    pub(crate) fn render(
        &self,
        buf: &mut Buffer,
        theme: &Theme,
        title: &str,
        body_lines: u16,
        min_height: u16,
    ) -> (Rect, Rect) {
        let Self {
            backdrop,
            available,
            width,
            padding_x,
            padding_y,
        } = *self;
        let height = body_lines
            .saturating_add(4 + 2 * padding_y)
            .max(min_height)
            .min(available.height);
        let area = Rect::new(
            available.x + (available.width - width) / 2,
            available.y + (available.height - height) / 2,
            width,
            height,
        );
        for position in backdrop.positions() {
            let cell = &mut buf[position];
            cell.set_style(theme.backdrop_style(cell.fg, cell.bg));
        }
        Clear.render(area, buf);
        let block = Block::bordered()
            .style(theme.base)
            .border_style(theme.prompt.key)
            .title(Span::styled(format!(" {title} "), theme.application_title))
            .padding(Padding::new(padding_x, padding_x, padding_y, padding_y));
        let inner = block.inner(area);
        block.render(area, buf);
        let [body, _, footer] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(u16::from(inner.height > 5)),
            Constraint::Length(1),
        ])
        .areas(inner);
        (body, footer)
    }
}
