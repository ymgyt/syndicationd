use std::borrow::Cow;

use synd_client::payload;

use crate::{
    application::{Direction, Populate},
    types::{EntryExt, RequirementExt, TimeExt},
    ui::{
        self, Context, icon,
        widgets::{collections::FilterableVec, filter::FeedFilterer},
        widgets::{scrollbar::Scrollbar, table::Table},
    },
};
use ratatui::{
    prelude::{Alignment, Buffer, Constraint, Layout, Rect},
    style::Stylize,
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Cell, Padding, Paragraph, Row, Widget, Wrap},
};
use synd_feed::types::FeedUrl;

#[allow(clippy::struct_field_names)]
pub(crate) struct EntriesWidget {
    entries: FilterableVec<payload::TimelineEntry, FeedFilterer>,
}

impl EntriesWidget {
    pub(crate) fn new() -> Self {
        Self {
            entries: FilterableVec::new(),
        }
    }

    pub(crate) fn update_timeline_chunk(
        &mut self,
        populate: Populate,
        entries: Vec<payload::TimelineEntry>,
        limit: usize,
    ) {
        self.entries.update(populate, entries);
        self.entries.truncate(limit);
    }

    /// Apply timeline changes in seq order, keeping the display order.
    pub(crate) fn apply_changes(&mut self, changes: Vec<payload::TimelineChange>, limit: usize) {
        for change in changes {
            match change {
                payload::TimelineChange::Upsert { timeline_entry } => {
                    self.entries
                        .upsert_sorted(*timeline_entry, Self::display_order, |a, b| {
                            a.entry.id == b.entry.id
                        });
                }
                payload::TimelineChange::Remove { entry_id } => {
                    self.entries.retain(|entry| entry.entry.id != entry_id);
                }
            }
        }
        self.entries.truncate(limit);
    }

    /// Timeline display order: `(order_time, entry.id)` descending.
    fn display_order(a: &payload::TimelineEntry, b: &payload::TimelineEntry) -> std::cmp::Ordering {
        b.order_time
            .cmp(&a.order_time)
            .then_with(|| b.entry.id.cmp(&a.entry.id))
    }

    pub(crate) fn update_filterer(&mut self, filterer: FeedFilterer) {
        self.entries.update_filter(filterer);
    }

    pub(crate) fn remove_unsubscribed_entries(&mut self, url: &FeedUrl) {
        self.entries.retain(|entry| &entry.entry.feed.url != url);
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

    pub(crate) fn entries(&self) -> impl Iterator<Item = &payload::Entry> {
        self.entries
            .as_unfiltered_slice()
            .iter()
            .map(|entry| &entry.entry)
    }

    pub(crate) fn selected_entry_website_url(&self) -> Option<&str> {
        self.selected_entry()
            .and_then(|entry| entry.website_url.as_deref())
    }

    fn selected_entry(&self) -> Option<&payload::Entry> {
        self.entries.selected().map(|entry| &entry.entry)
    }
}

impl EntriesWidget {
    pub fn render(&self, area: Rect, buf: &mut Buffer, cx: &Context<'_>) {
        let vertical = Layout::vertical([Constraint::Fill(2), Constraint::Fill(1)]);
        let [entries_area, detail_area] = vertical.areas(area);

        self.render_entries(entries_area, buf, cx);
        self.render_detail(detail_area, buf, cx);
    }

    fn render_entries(&self, area: Rect, buf: &mut Buffer, cx: &Context<'_>) {
        let entries_area = Block::new().padding(Padding::top(1)).inner(area);

        let (header, widths, rows) = self.entry_rows(cx);

        Table::builder()
            .header(header)
            .widths(widths)
            .rows(rows)
            .theme(&cx.theme.entries)
            .selected_idx(self.entries.selected_index())
            .highlight_modifier(cx.table_highlight_modifier())
            .build()
            .render(entries_area, buf);

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

        let row = |timeline_entry: &'a payload::TimelineEntry| {
            let entry = &timeline_entry.entry;
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

            let feed_title = entry.feed.title.as_deref().unwrap_or(ui::UNKNOWN_SYMBOL);
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
        let [
            title_area,
            url_area,
            published_area,
            _,
            summary_heading_area,
            summary_area,
        ] = vertical.areas(inner);

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
