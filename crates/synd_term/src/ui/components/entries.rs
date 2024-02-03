use ratatui::{
    prelude::{Alignment, Buffer, Constraint, Layout, Margin, Rect},
    text::{Span, Text},
    widgets::{
        block::{Position, Title},
        Block, BorderType, Borders, Cell, Padding, Paragraph, Row, Scrollbar, ScrollbarOrientation,
        ScrollbarState, StatefulWidget, Table, TableState, Widget, Wrap,
    },
};
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    application::{Direction, IndexOutOfRange},
    client::payload,
    types::{self, TimeExt},
    ui::{self, Context},
};

pub struct Entries {
    selected_entry_index: usize,
    entries: Vec<types::Entry>,
}

impl Entries {
    pub fn new() -> Self {
        Self {
            selected_entry_index: 0,
            entries: Vec::new(),
        }
    }

    pub fn update_entries(&mut self, payload: payload::FetchEntriesPayload) {
        self.entries.extend(payload.entries);
    }

    pub fn move_selection(&mut self, direction: &Direction) {
        self.selected_entry_index = direction.apply(
            self.selected_entry_index,
            self.entries.len(),
            IndexOutOfRange::Wrapping,
        );
    }

    pub fn selected_entry_website_url(&self) -> Option<&str> {
        self.selected_entry()
            .and_then(|entry| entry.website_url.as_deref())
    }

    fn selected_entry(&self) -> Option<&types::Entry> {
        self.entries.get(self.selected_entry_index)
    }
}

impl Entries {
    pub fn render(&self, area: Rect, buf: &mut Buffer, cx: &Context<'_>) {
        let vertical = Layout::vertical([Constraint::Percentage(70), Constraint::Percentage(30)]);
        let [entries_area, summary_area] = vertical.areas(area);

        self.render_entries(entries_area, buf, cx);
        self.render_summary(summary_area, buf, cx);
    }

    fn render_entries(&self, area: Rect, buf: &mut Buffer, cx: &Context<'_>) {
        // padding
        let entries_area = area.inner(&Margin {
            vertical: 1,
            horizontal: 1,
        });

        let mut entries_state = TableState::new()
            .with_offset(0)
            .with_selected(self.selected_entry_index);

        let (header, widths, rows) = self.entry_rows(cx);

        let entries = Table::new(rows, widths)
            .header(header.style(cx.theme.entries.header))
            .column_spacing(2)
            .style(cx.theme.entries.background)
            .highlight_symbol(ui::TABLE_HIGHLIGHT_SYMBOL)
            .highlight_style(cx.theme.entries.selected_entry)
            .highlight_spacing(ratatui::widgets::HighlightSpacing::WhenSelected);

        StatefulWidget::render(entries, entries_area, buf, &mut entries_state);

        let scrollbar_area = Rect {
            y: area.y + 2, // table header
            height: area.height - 3,
            ..area
        };

        let mut scrollbar_state = ScrollbarState::default()
            .content_length(self.entries.len())
            .position(self.selected_entry_index);
        Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .track_symbol(None)
            .thumb_symbol("▐")
            .render(scrollbar_area, buf, &mut scrollbar_state);
    }

    fn entry_rows<'a>(
        &'a self,
        _cx: &'a Context<'_>,
    ) -> (
        Row<'a>,
        impl IntoIterator<Item = Constraint>,
        impl IntoIterator<Item = Row<'a>>,
    ) {
        let title_width = self
            .entries
            .iter()
            .filter_map(|entry| entry.title.as_deref())
            .map(|title| title.graphemes(true).count())
            .max()
            .unwrap_or(10)
            .max(60)
            .try_into()
            .unwrap_or(60);

        let header = Row::new([
            Cell::from("Published"),
            Cell::from("Title"),
            Cell::from("Feed"),
        ]);

        let constraints = [
            Constraint::Length(10),
            Constraint::Min(70),
            Constraint::Length(title_width),
        ];

        let row = |entry: &'a types::Entry| {
            let title = entry.title.as_deref().unwrap_or(ui::UNKNOWN_SYMBOL);
            let published = entry
                .published
                .as_ref()
                .map_or_else(|| ui::UNKNOWN_SYMBOL.to_string(), TimeExt::local_ymd);

            let feed_title = entry.feed_title.as_deref().unwrap_or(ui::UNKNOWN_SYMBOL);

            Row::new([
                Cell::from(Span::from(published)),
                Cell::from(Span::from(title)),
                Cell::from(Span::from(feed_title)),
            ])
        };

        (header, constraints, self.entries.iter().map(row))
    }

    fn render_summary(&self, area: Rect, buf: &mut Buffer, _cx: &Context<'_>) {
        let block = Block::new()
            .padding(Padding {
                left: 1,
                right: 1,
                top: 1,
                bottom: 1,
            })
            .title(
                Title::from("Summary")
                    .position(Position::Top)
                    .alignment(Alignment::Center),
            )
            .borders(Borders::TOP)
            .border_type(BorderType::Thick);

        let inner = block.inner(area);
        Widget::render(block, area, buf);

        let Some(entry) = self.selected_entry() else {
            return;
        };
        let Some(summary) = entry.summary_text(inner.width.into()) else {
            return;
        };
        // should to Lines?
        let paragraph = Paragraph::new(Text::from(summary))
            .wrap(Wrap { trim: false })
            .alignment(Alignment::Center);

        Widget::render(paragraph, inner, buf);
    }
}
