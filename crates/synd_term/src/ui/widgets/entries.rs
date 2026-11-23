use std::borrow::Cow;

use synd_client::payload;

use crate::{
    application::Populate,
    command::ListCommand,
    types::{EntryExt, TimeExt, format_local},
    ui::{
        self, Context, icon,
        widgets::{collections::FilterableVec, filter::FeedFilterer},
        widgets::{scrollbar::Scrollbar, table::Table},
    },
};
use ratatui::{
    prelude::{Buffer, Constraint, Layout, Rect},
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

    pub(crate) fn apply_list(&mut self, command: ListCommand) {
        self.entries.apply_list(command);
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
        // The scrollbar occupies the rightmost cell beside the date.
        let entries_area = Block::new().padding(Padding::new(0, 1, 1, 0)).inner(area);

        let (header, widths, rows) = self.entry_rows(entries_area.width, cx);

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
        table_width: u16,
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
            Cell::from(format!("Entry {n}/{m}")),
            Cell::from("Feed"),
            Cell::from(Line::from("Date").right_aligned()),
        ]);

        let feed_limit = (table_width / 4).min(30);
        let mut feed_width = 4.min(feed_limit); // "Feed"
        for name in self
            .entries
            .iter()
            .filter_map(|e| e.entry.feed.title.as_deref())
        {
            let width = u16::try_from(Span::raw(name).width()).unwrap_or(feed_limit);
            feed_width = feed_width.max(width).min(feed_limit);
            if feed_width == feed_limit {
                break;
            }
        }
        let constraints = [
            Constraint::Fill(1),
            Constraint::Length(feed_width),
            Constraint::Length(5),
        ];

        let row = |timeline_entry: &'a payload::TimelineEntry| {
            let entry = &timeline_entry.entry;
            let title = entry.title.as_deref().unwrap_or(ui::UNKNOWN_SYMBOL);
            let published = entry
                .published
                .as_ref()
                .or(entry.updated.as_ref())
                .map_or_else(
                    || ui::UNKNOWN_SYMBOL.to_owned(),
                    |time| format_local(time, "%m/%d"),
                );
            let category = entry.category();
            let icon = cx
                .categories
                .icon(category)
                .unwrap_or_else(|| ui::default_icon());

            let feed_title = entry.feed.title.as_deref().unwrap_or(ui::UNKNOWN_SYMBOL);

            Row::new([
                Cell::from(Line::from(vec![
                    Span::from(icon.symbol()).fg(icon.color().unwrap_or(cx.theme.default_icon_fg)),
                    Span::from(" "),
                    Span::from(title),
                ])),
                Cell::from(Span::from(feed_title)),
                Cell::from(Line::from(published).right_aligned()),
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

        if inner.is_empty() {
            return;
        }

        let mut metadata = Line::default().dim();
        if let Some(time) = entry.published.as_ref().or(entry.updated.as_ref()) {
            metadata.push_span(concat!(icon!(calendar), " "));
            metadata.push_span(time.local_ymd_hm());
        }
        if let Some(url) = entry.website_url.as_deref().filter(|url| !url.is_empty()) {
            if !metadata.spans.is_empty() {
                metadata.push_span("  ");
            }
            metadata.push_span(concat!(icon!(open), " "));
            metadata.push_span(url);
        }

        let show_metadata = inner.height > 1 && !metadata.spans.is_empty();
        let [body, footer] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(u16::from(show_metadata)),
        ])
        .areas(inner);

        // Let the paragraph wrap the visible text together with its first-line label.
        let summary = entry.summary_text(usize::from(body.width) * usize::from(body.height));
        let (label, content) = match summary.as_deref().filter(|text| !text.trim().is_empty()) {
            Some(text) => (concat!(icon!(summary), " "), text),
            None => (
                concat!(icon!(entry), " Entry  "),
                entry.title.as_deref().unwrap_or(ui::UNKNOWN_SYMBOL),
            ),
        };
        let mut text = Text::from(content.trim_matches('\n'));
        if let Some(first) = text.lines.first_mut() {
            first.spans.insert(0, Span::raw(label).bold());
        }
        Paragraph::new(text)
            .wrap(Wrap { trim: false })
            .style(cx.theme.entries.summary)
            .render(body, buf);

        if show_metadata {
            let clipped = metadata.width() > usize::from(footer.width);
            let (x, y) = buf.set_line(
                footer.x,
                footer.y,
                &metadata,
                footer.width - u16::from(clipped),
            );
            if clipped {
                buf.set_string(x, y, "…", metadata.style);
            }
        }
    }
}
