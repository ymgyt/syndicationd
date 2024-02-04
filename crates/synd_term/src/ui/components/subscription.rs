use ratatui::{
    prelude::{Alignment, Buffer, Constraint, Layout, Margin, Rect},
    text::Span,
    widgets::{
        block::{Position, Title},
        Block, BorderType, Borders, Cell, HighlightSpacing, Padding, Row, Scrollbar,
        ScrollbarOrientation, ScrollbarState, StatefulWidget, Table, TableState, Widget,
    },
};
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    application::{Direction, IndexOutOfRange},
    client::query::subscription::SubscriptionOutput,
    types::{self, EntryMeta, Feed, TimeExt},
    ui::{self, Context},
};

pub struct Subscription {
    selected_feed_meta_index: usize,
    feed_metas: Vec<types::Feed>,
}

impl Subscription {
    pub fn new() -> Self {
        Self {
            selected_feed_meta_index: 0,
            feed_metas: Vec::new(),
        }
    }

    pub fn has_subscription(&self) -> bool {
        !self.feed_metas.is_empty()
    }

    pub fn selected_feed_website_url(&self) -> Option<&str> {
        self.feed_metas
            .get(self.selected_feed_meta_index)
            .and_then(|feed_meta| feed_meta.website_url.as_deref())
    }

    pub fn selected_feed_url(&self) -> Option<&str> {
        self.feed_metas
            .get(self.selected_feed_meta_index)
            .map(|feed_meta| feed_meta.url.as_str())
    }

    pub fn update_subscription(&mut self, subscription: SubscriptionOutput) {
        let feed_metas = subscription.feeds.nodes.into_iter().map(types::Feed::from);
        self.feed_metas = feed_metas.collect();
    }

    pub fn add_subscribed_feed(&mut self, feed: types::Feed) {
        self.feed_metas.insert(0, feed);
    }

    pub fn remove_unsubscribed_feed(&mut self, url: &str) {
        self.feed_metas.retain(|feed_meta| feed_meta.url != url);
        self.move_selection(&Direction::Up);
    }

    pub fn move_selection(&mut self, direction: &Direction) {
        self.selected_feed_meta_index = direction.apply(
            self.selected_feed_meta_index,
            self.feed_metas.len(),
            IndexOutOfRange::Wrapping,
        );
    }
}

impl Subscription {
    pub fn render(&self, area: Rect, buf: &mut Buffer, cx: &Context<'_>) {
        let vertical = Layout::vertical([Constraint::Percentage(70), Constraint::Percentage(30)]);
        let [feeds_area, feed_entries_area] = vertical.areas(area);

        self.render_feeds(feeds_area, buf, cx);
        self.render_feed_entries(feed_entries_area, buf, cx);
    }

    fn render_feeds(&self, area: Rect, buf: &mut Buffer, cx: &Context<'_>) {
        // padding
        let feeds_area = area.inner(&Margin {
            vertical: 1,
            horizontal: 1,
        });

        let mut feeds_state = TableState::new()
            .with_offset(0)
            .with_selected(self.selected_feed_meta_index);

        let (header, widths, rows) = self.feed_rows(cx);

        let feeds = Table::new(rows, widths)
            .header(header.style(cx.theme.subscription.header))
            .column_spacing(2)
            .style(cx.theme.subscription.background)
            .highlight_symbol(ui::TABLE_HIGHLIGHT_SYMBOL)
            .highlight_style(cx.theme.subscription.selected_feed)
            .highlight_spacing(HighlightSpacing::WhenSelected);

        StatefulWidget::render(feeds, feeds_area, buf, &mut feeds_state);

        let scrollbar_area = Rect {
            y: area.y + 2, // table header
            height: area.height - 3,
            ..area
        };

        // https://github.com/ratatui-org/ratatui/pull/911
        // passing None to track_symbol cause incorrect rendering
        let mut scrollbar_state = ScrollbarState::default()
            .content_length(self.feed_metas.len())
            .position(self.selected_feed_meta_index);
        Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .track_symbol(Some(" "))
            .thumb_symbol("▐")
            .render(scrollbar_area, buf, &mut scrollbar_state);
    }

    fn feed_rows<'a>(
        &'a self,
        _cx: &'a Context<'_>,
    ) -> (
        Row<'a>,
        impl IntoIterator<Item = Constraint>,
        impl IntoIterator<Item = Row<'a>>,
    ) {
        let header = Row::new([
            Cell::from("Feed"),
            Cell::from("Updated"),
            Cell::from("URL"),
            Cell::from("Description"),
        ]);

        let constraints = [
            Constraint::Fill(1),
            Constraint::Length(10),
            Constraint::Fill(1),
            Constraint::Fill(2),
        ];

        let row = |feed_meta: &'a Feed| {
            let title = feed_meta.title.as_deref().unwrap_or(ui::UNKNOWN_SYMBOL);
            let updated = feed_meta
                .updated
                .as_ref()
                .map_or_else(|| ui::UNKNOWN_SYMBOL.to_string(), TimeExt::local_ymd);
            let desc = feed_meta.description.as_deref().unwrap_or("");
            let website_url = feed_meta
                .website_url
                .as_deref()
                .unwrap_or(ui::UNKNOWN_SYMBOL);

            Row::new([
                Cell::from(Span::from(title)),
                Cell::from(Span::from(updated)),
                Cell::from(Span::from(website_url)),
                Cell::from(Span::from(desc)),
            ])
        };

        (header, constraints, self.feed_metas.iter().map(row))
    }

    fn render_feed_entries(&self, area: Rect, buf: &mut Buffer, cx: &Context<'_>) {
        let block = Block::new()
            .padding(Padding {
                left: 1,
                right: 1,
                top: 1,
                bottom: 1,
            })
            .title(
                Title::from("Latest 10 entries")
                    .position(Position::Top)
                    .alignment(Alignment::Center),
            )
            .borders(Borders::TOP)
            .border_type(BorderType::Thick);

        let inner = block.inner(area);
        Widget::render(block, area, buf);

        let Some(feed) = self.feed_metas.get(self.selected_feed_meta_index) else {
            return;
        };

        let title_width = feed
            .entries
            .iter()
            .filter_map(|entry| entry.title.as_deref())
            .map(|title| title.graphemes(true).count())
            .max()
            .unwrap_or(0);

        let entry = |entry: &EntryMeta| {
            let title = entry.title.as_deref().unwrap_or(ui::UNKNOWN_SYMBOL);
            let published = entry
                .published
                .as_ref()
                .map_or_else(|| ui::UNKNOWN_SYMBOL.to_string(), TimeExt::local_ymd);
            let summary = entry.summary_text(100).unwrap_or(ui::UNKNOWN_SYMBOL.into());

            Row::new([
                Cell::new(Span::from(published)),
                Cell::new(Span::from(title.to_string())),
                Cell::new(Span::from(summary)),
            ])
        };

        let header = Row::new([
            Cell::new(Span::from("Published")),
            Cell::new(Span::from("Entry")),
            Cell::new(Span::from("Summary")),
        ]);

        let widths = [
            Constraint::Length(10),
            Constraint::Length(title_width.try_into().unwrap_or(100)),
            Constraint::Max(200),
        ];
        let rows = feed.entries.iter().map(entry);
        let table = Table::new(rows, widths)
            .header(header.style(cx.theme.subscription.header))
            .column_spacing(2)
            .style(cx.theme.subscription.background);

        Widget::render(table, inner, buf);
    }
}
