use std::borrow::Cow;

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Flex, Layout, Rect},
    style::Stylize,
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Cell, Padding, Paragraph, Row, Widget, Wrap},
};
use synd_client::payload;
use synd_feed::{entry::SourceCategory, types::FeedUrl};

use crate::{
    application::Populate,
    command::ListCommand,
    types::{EntryExt, TimeExt},
    ui::{
        self, Context, icon,
        widgets::{
            collections::FilterableVec,
            filter::FeedFilterer,
            scrollbar::Scrollbar,
            table::{self, Table},
        },
    },
};

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

    pub(crate) fn selected_entry(&self) -> Option<&payload::Entry> {
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
        let entries_area = Table::inner_area(area);

        let (header, widths, rows) = self.entry_rows(entries_area.width, cx);

        Table::builder()
            .header(header)
            .widths(widths)
            .rows(rows)
            .selected_idx(self.entries.selected_index())
            .build(cx)
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
            table::DATE_WIDTH,
        ];

        let row = |(timeline_entry, selected): (&'a payload::TimelineEntry, bool)| {
            let entry = &timeline_entry.entry;
            let title = entry.title.as_deref().unwrap_or(ui::UNKNOWN_SYMBOL);
            let category = entry.category();
            let icon = cx
                .categories
                .icon(category)
                .unwrap_or_else(|| ui::default_icon());

            let feed_title = entry.feed.title.as_deref().unwrap_or(ui::UNKNOWN_SYMBOL);

            Row::new([
                table::title_cell(
                    Span::from(icon.symbol()).fg(icon.color().unwrap_or(cx.theme.default_icon_fg)),
                    title,
                    selected,
                    cx,
                ),
                Cell::from(Span::from(feed_title)),
                table::date_cell(entry.published.as_ref().or(entry.updated.as_ref())),
            ])
        };

        (
            header,
            constraints,
            self.entries.iter_with_selection().map(row),
        )
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

        let has_categories = !entry.source_categories.is_empty();
        let show_metadata =
            !metadata.spans.is_empty() && inner.height > 1 + u16::from(has_categories);
        let [body, footer] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(u16::from(show_metadata)),
        ])
        .areas(inner);

        let summary = summary_paragraph(entry).style(cx.theme.entries.summary);
        if has_categories {
            let lines = u16::try_from(summary.line_count(body.width)).unwrap_or(u16::MAX);
            let [summary_area, category_area] =
                Layout::vertical([Constraint::Length(lines), Constraint::Min(1)]).areas(body);
            summary.render(summary_area, buf);
            render_categories(&entry.source_categories, category_area, buf);
        } else {
            summary.render(body, buf);
        }

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

fn summary_paragraph(entry: &payload::Entry) -> Paragraph<'_> {
    let (label, content) = match &entry.excerpt {
        Some(excerpt) if !excerpt.text.trim().is_empty() => {
            (concat!(icon!(summary), " "), excerpt.text.as_str())
        }
        _ => (
            concat!(icon!(entry), " Entry  "),
            entry.title.as_deref().unwrap_or(ui::UNKNOWN_SYMBOL),
        ),
    };
    let mut text = Text::from(content.trim_matches('\n'));
    if let Some(first) = text.lines.first_mut() {
        first.spans.insert(0, Span::raw(label).bold());
    }
    Paragraph::new(text).wrap(Wrap { trim: false })
}

fn render_categories(categories: &[SourceCategory], area: Rect, buf: &mut Buffer) {
    let terms: Vec<_> = categories.iter().map(display_category_term).collect();
    let paragraph = |visible: usize| {
        let mut line = Line::from(icon!(tag));
        for term in &terms[..visible] {
            line.push_span(" ");
            line.push_span(term.as_ref());
        }
        if visible < terms.len() {
            line.push_span(format!(" … +{}", terms.len() - visible));
        }
        let paragraph = Paragraph::new(line).dim().wrap(Wrap { trim: true });
        let lines = paragraph.line_count(area.width);
        (paragraph, lines)
    };
    let height = usize::from(area.height);
    let (all, all_lines) = paragraph(terms.len());
    let fitting = if all_lines <= height {
        Some((all, all_lines))
    } else {
        // Include the omission count when measuring the visible terms.
        (1..terms.len())
            .map(paragraph)
            .take_while(|(_, lines)| *lines <= height)
            .last()
    };
    if let Some((candidate, lines)) = fitting {
        let [bottom] = Layout::vertical([Constraint::Length(
            u16::try_from(lines).unwrap_or(area.height),
        )])
        .flex(Flex::End)
        .areas(area);
        candidate.render(bottom, buf);
        return;
    }

    // Even one complete term would not fit. Keep its prefix visible beside the marker.
    let omitted = terms.len() - 1;
    let marker = if omitted == 0 {
        "…".to_owned()
    } else {
        format!("… +{omitted}")
    };
    let marker_width = u16::try_from(Span::raw(&marker).width()).unwrap_or(u16::MAX);
    let [term, tail] =
        Layout::horizontal([Constraint::Min(1), Constraint::Length(marker_width)]).areas(area);
    Paragraph::new(terms[0].as_ref())
        .dim()
        .wrap(Wrap { trim: true })
        .render(term, buf);
    if let Some(last) = tail.rows().next_back() {
        Line::from(marker).dim().render(last, buf);
    }
}

fn display_category_term(category: &SourceCategory) -> Cow<'_, str> {
    let term = category.term.as_str();
    if term.contains(char::is_whitespace) {
        Cow::Owned(serde_json::to_string(term).expect("a string always serializes to JSON"))
    } else {
        Cow::Borrowed(term)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::screen::Screen;

    #[test]
    fn multiword_category_quotes_count_toward_the_preview_width() {
        let categories = ["This Week in Rust", "Releases"].map(|term| SourceCategory {
            term: term.to_owned(),
            scheme: None,
            label: None,
        });
        let preview = |width, height| {
            let area = Rect::new(0, 0, width, height);
            let mut buffer = Buffer::empty(area);
            render_categories(&categories, area, &mut buffer);
            Screen::new(&buffer).lines()
        };

        assert_eq!(
            preview(30, 1),
            [concat!(icon!(tag), r#" "This Week in Rust" Releases"#)]
        );
        assert_eq!(
            preview(16, 2),
            [
                concat!(icon!(tag), r#" "This Week in"#),
                r#"Rust" Releases"#
            ]
        );
        assert_eq!(
            preview(29, 1),
            [concat!(icon!(tag), r#" "This Week in Rust" … +1"#)]
        );
        assert_eq!(preview(8, 1), [r#""Thi… +1"#]);
    }

    #[test]
    fn an_overlong_category_keeps_its_prefix_and_omission_count() {
        let categories = ["東京都市圏", "地域"].map(|term| SourceCategory {
            term: term.to_owned(),
            scheme: None,
            label: None,
        });
        let area = Rect::new(0, 0, 8, 1);
        let mut buffer = Buffer::empty(area);
        render_categories(&categories, area, &mut buffer);
        assert_eq!(buffer[(0, 0)].symbol(), "東");
        assert_eq!(buffer[(2, 0)].symbol(), "京");
        assert_eq!(buffer[(4, 0)].symbol(), "…");
        assert_eq!(buffer[(7, 0)].symbol(), "1");
    }
}
