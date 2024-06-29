use std::borrow::Cow;

use crate::{
    application::{Direction, Populate},
    client::payload,
    types::{self, RequirementExt, TimeExt},
    ui::{
        self,
        components::{collections::FilterableVec, filter::FeedFilterer},
        icon,
        widgets::scrollbar::Scrollbar,
        Context,
    },
};
use ratatui::{
    prelude::{Alignment, Buffer, Constraint, Layout, Rect},
    style::Stylize,
    text::{Line, Span, Text},
    widgets::{
        Block, BorderType, Borders, Cell, Padding, Paragraph, Row, StatefulWidget, Table,
        TableState, Widget, Wrap,
    },
};
use synd_feed::types::FeedUrl;

#[allow(clippy::struct_field_names)]
pub(crate) struct Entries {
    entries: FilterableVec<types::Entry, FeedFilterer>,
}

impl Entries {
    pub(crate) fn new() -> Self {
        Self {
            entries: FilterableVec::new(),
        }
    }

    /// Return entries count
    pub(crate) fn count(&self) -> usize {
        self.entries.len()
    }

    pub(crate) fn update_entries(
        &mut self,
        populate: Populate,
        payload: payload::FetchEntriesPayload,
    ) {
        self.entries.update(populate, payload.entries);
    }

    pub(crate) fn update_filterer(&mut self, filterer: FeedFilterer) {
        self.entries.update_filter(filterer);
    }

    pub(crate) fn remove_unsubscribed_entries(&mut self, url: &FeedUrl) {
        self.entries.retain(|entry| &entry.feed_url != url);
    }

    pub(crate) fn move_selection(&mut self, direction: Direction) {
        self.entries.move_selection(direction);
    }

    pub(crate) fn move_first(&mut self) {
        self.entries.move_first();
    }

    pub(crate) fn move_last(&mut self) {
        self.entries.move_last();
    }

    pub(crate) fn entries(&self) -> &[types::Entry] {
        self.entries.as_unfiltered_slice()
    }

    pub(crate) fn selected_entry_website_url(&self) -> Option<&str> {
        self.entries
            .selected()
            .and_then(|entry| entry.website_url.as_deref())
    }

    fn selected_entry(&self) -> Option<&types::Entry> {
        self.entries.selected()
    }
}

impl Entries {
    pub fn render(&self, area: Rect, buf: &mut Buffer, cx: &Context<'_>) {
        let vertical = Layout::vertical([Constraint::Fill(2), Constraint::Fill(1)]);
        let [entries_area, detail_area] = vertical.areas(area);

        self.render_entries(entries_area, buf, cx);
        self.render_detail(detail_area, buf, cx);
    }

    fn render_entries(&self, area: Rect, buf: &mut Buffer, cx: &Context<'_>) {
        let entries_area = Block::new().padding(Padding::top(1)).inner(area);

        let mut entries_state = TableState::new()
            .with_offset(0)
            .with_selected(self.entries.selected_index());

        let (header, widths, rows) = self.entry_rows(cx);

        let entries = Table::new(rows, widths)
            .header(header.style(cx.theme.entries.header))
            .column_spacing(2)
            .highlight_symbol(ui::TABLE_HIGHLIGHT_SYMBOL)
            .highlight_style(
                cx.theme
                    .entries
                    .selected_entry
                    .add_modifier(cx.table_highlight_modifier()),
            )
            .highlight_spacing(ratatui::widgets::HighlightSpacing::WhenSelected);

        StatefulWidget::render(entries, entries_area, buf, &mut entries_state);

        let header_rows = 2;
        #[allow(clippy::cast_possible_truncation)]
        let scrollbar_area = Rect {
            y: area.y + header_rows, // table header
            height: area
                .height
                .saturating_sub(header_rows)
                .min(self.entries.len() as u16),
            ..area
        };

        Scrollbar {
            content_length: self.entries.len(),
            position: self.entries.selected_index(),
        }
        .render(scrollbar_area, buf, cx);
    }

    fn entry_rows<'a>(
        &'a self,
        cx: &'a Context<'_>,
    ) -> (
        Row<'a>,
        impl IntoIterator<Item = Constraint>,
        impl IntoIterator<Item = Row<'a>>,
    ) {
        let (n, m) = {
            if self.entries.is_empty() {
                (Cow::Borrowed("-"), Cow::Borrowed("-"))
            } else {
                (
                    Cow::Owned((self.entries.selected_index() + 1).to_string()),
                    Cow::Owned(self.entries.len().to_string()),
                )
            }
        };
        let header = Row::new([
            Cell::from("Published"),
            Cell::from(format!("Entry {n}/{m}")),
            Cell::from("Feed"),
            Cell::from("Req"),
        ]);

        let constraints = [
            Constraint::Length(10),
            Constraint::Fill(2),
            Constraint::Fill(1),
            Constraint::Length(4),
        ];

        let row = |entry: &'a types::Entry| {
            let title = entry.title.as_deref().unwrap_or(ui::UNKNOWN_SYMBOL);
            let published = entry
                .published
                .as_ref()
                .or(entry.updated.as_ref())
                .map_or_else(|| ui::UNKNOWN_SYMBOL.to_string(), TimeExt::local_ymd);
            let category = entry.category();
            let icon = cx
                .categories
                .icon(category)
                .unwrap_or_else(|| ui::default_icon());

            let feed_title = entry.feed_title.as_deref().unwrap_or(ui::UNKNOWN_SYMBOL);
            let requirement = entry.requirement().label(&cx.theme.requirement);

            Row::new([
                Cell::from(Span::from(published)),
                Cell::from(Line::from(vec![
                    Span::from(icon.symbol()).fg(icon.color().unwrap_or(cx.theme.default_icon_fg)),
                    Span::from(" "),
                    Span::from(title),
                ])),
                Cell::from(Span::from(feed_title)),
                Cell::from(Line::from(vec![requirement, Span::from(" ")])),
            ])
        };

        (header, constraints, self.entries.iter().map(row))
    }

    fn render_detail(&self, area: Rect, buf: &mut Buffer, cx: &Context<'_>) {
        let block = Block::new()
            .padding(Padding::horizontal(2))
            .borders(Borders::TOP)
            .border_type(BorderType::Plain);

        let inner = block.inner(area);
        Widget::render(block, area, buf);

        let Some(entry) = self.selected_entry() else {
            return;
        };

        let vertical = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ]);
        let [title_area, url_area, published_area, _, summary_heading_area, summary_area] =
            vertical.areas(inner);

        Line::from(vec![
            Span::from(concat!(icon!(entry), " Entry")).bold(),
            Span::from("     "),
            Span::from(entry.title.as_deref().unwrap_or(ui::UNKNOWN_SYMBOL)),
        ])
        .render(title_area, buf);

        Line::from(vec![
            Span::from(concat!(icon!(open), " URL")).bold(),
            Span::from("       "),
            Span::from(entry.website_url.as_deref().unwrap_or_default()),
        ])
        .render(url_area, buf);

        Line::from(vec![
            Span::from(concat!(icon!(calendar), " Published")).bold(),
            Span::from(" "),
            Span::from(
                entry
                    .published
                    .as_ref()
                    .or(entry.updated.as_ref())
                    .map_or_else(|| ui::UNKNOWN_SYMBOL.to_string(), TimeExt::local_ymd_hm),
            ),
        ])
        .render(published_area, buf);

        let Some(summary) = entry.summary_text(inner.width.into()) else {
            return;
        };

        Line::from(
            Span::from(concat!(icon!(summary), " Summary"))
                .bold()
                .underlined(),
        )
        .render(summary_heading_area, buf);

        let paragraph = Paragraph::new(Text::from(summary))
            .wrap(Wrap { trim: false })
            .style(cx.theme.entries.summary)
            .alignment(Alignment::Left);

        Widget::render(paragraph, summary_area, buf);
    }
}
