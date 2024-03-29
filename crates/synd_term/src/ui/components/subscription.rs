use std::borrow::Cow;

use itertools::Itertools;
use ratatui::{
    prelude::{Alignment, Buffer, Constraint, Layout, Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        block::{Position, Title},
        Block, BorderType, Borders, Cell, HighlightSpacing, Padding, Row, Scrollbar,
        ScrollbarOrientation, ScrollbarState, StatefulWidget, Table, TableState, Widget,
    },
};
use synd_feed::types::FeedType;

use crate::{
    application::{Direction, IndexOutOfRange, ListAction},
    client::query::subscription::SubscriptionOutput,
    types::{self, EntryMeta, Feed, TimeExt},
    ui::{self, Context},
};

pub struct Subscription {
    selected_feed_meta_index: usize,
    feeds: Vec<types::Feed>,
}

impl Subscription {
    pub fn new() -> Self {
        Self {
            selected_feed_meta_index: 0,
            feeds: Vec::new(),
        }
    }

    pub fn has_subscription(&self) -> bool {
        !self.feeds.is_empty()
    }

    pub fn is_already_subscribed(&self, url: &str) -> bool {
        self.feeds.iter().any(|feed| feed.url == url)
    }

    pub fn selected_feed_website_url(&self) -> Option<&str> {
        self.feeds
            .get(self.selected_feed_meta_index)
            .and_then(|feed_meta| feed_meta.website_url.as_deref())
    }

    pub fn selected_feed_url(&self) -> Option<&str> {
        self.feeds
            .get(self.selected_feed_meta_index)
            .map(|feed_meta| feed_meta.url.as_str())
    }

    pub fn update_subscription(&mut self, action: ListAction, subscription: SubscriptionOutput) {
        let feed_metas = subscription.feeds.nodes.into_iter().map(types::Feed::from);
        match action {
            ListAction::Append => self.feeds.extend(feed_metas),
            ListAction::Replace => self.feeds = feed_metas.collect(),
        }
    }

    pub fn add_subscribed_feed(&mut self, feed: types::Feed) {
        self.feeds.insert(0, feed);
    }

    pub fn remove_unsubscribed_feed(&mut self, url: &str) {
        self.feeds.retain(|feed_meta| feed_meta.url != url);
        self.move_selection(&Direction::Up);
    }

    pub fn move_selection(&mut self, direction: &Direction) {
        self.selected_feed_meta_index = direction.apply(
            self.selected_feed_meta_index,
            self.feeds.len(),
            IndexOutOfRange::Wrapping,
        );
    }

    pub fn move_first(&mut self) {
        self.selected_feed_meta_index = 0;
    }

    pub fn move_last(&mut self) {
        if !self.feeds.is_empty() {
            self.selected_feed_meta_index = self.feeds.len() - 1;
        }
    }
}

impl Subscription {
    pub fn render(&self, area: Rect, buf: &mut Buffer, cx: &Context<'_>) {
        let vertical = Layout::vertical([Constraint::Fill(2), Constraint::Fill(1)]);
        let [feeds_area, feed_detail_area] = vertical.areas(area);

        self.render_feeds(feeds_area, buf, cx);
        self.render_feed_detail(feed_detail_area, buf, cx);
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
            .block(Block::new().padding(Padding {
                left: 0,
                right: 0,
                top: 0,
                bottom: 0,
            }))
            .header(header.style(cx.theme.subscription.header))
            .column_spacing(2)
            .style(cx.theme.subscription.background)
            .highlight_symbol(ui::TABLE_HIGHLIGHT_SYMBOL)
            .highlight_style(cx.theme.subscription.selected_feed)
            .highlight_spacing(HighlightSpacing::WhenSelected);

        StatefulWidget::render(feeds, feeds_area, buf, &mut feeds_state);

        let scrollbar_area = Rect {
            y: area.y + 2, // table header
            height: area.height.saturating_sub(3),
            ..area
        };

        // https://github.com/ratatui-org/ratatui/pull/911
        // passing None to track_symbol cause incorrect rendering
        let mut scrollbar_state = ScrollbarState::default()
            .content_length(self.feeds.len())
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
            Cell::from("󰑫 Feed"),
            Cell::from(" Updated"),
            Cell::from(" URL"),
            Cell::from("󰎞 Description"),
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
                .or(feed_meta
                    .entries
                    .first()
                    .and_then(|entry| entry.published.as_ref().or(entry.updated.as_ref())))
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

        (header, constraints, self.feeds.iter().map(row))
    }

    #[allow(clippy::too_many_lines)]
    fn render_feed_detail(&self, area: Rect, buf: &mut Buffer, cx: &Context<'_>) {
        let block = Block::new()
            .padding(Padding {
                left: 3,
                right: 3,
                top: 1,
                bottom: 0,
            })
            .title(
                Title::from("Feed Detail")
                    .position(Position::Top)
                    .alignment(Alignment::Center),
            )
            .borders(Borders::TOP)
            .border_type(BorderType::Plain);

        let inner = block.inner(area);
        Widget::render(block, area, buf);

        let Some(feed) = self.feeds.get(self.selected_feed_meta_index) else {
            return;
        };

        let vertical = Layout::vertical([Constraint::Length(2), Constraint::Min(0)]);
        let [meta_area, entries_area] = vertical.areas(inner);
        let entries_area = entries_area.inner(&Margin {
            vertical: 1,
            horizontal: 0,
        });

        let widths = [
            Constraint::Length(11),
            Constraint::Fill(1),
            Constraint::Fill(2),
        ];

        let meta_rows = vec![
            Row::new([
                Cell::new(Span::styled(
                    "󰚼 Authors",
                    Style::default().add_modifier(Modifier::BOLD),
                )),
                Cell::new(Span::from(if feed.authors.is_empty() {
                    Cow::Borrowed(ui::UNKNOWN_SYMBOL)
                } else {
                    Cow::Owned(feed.authors.iter().join(", "))
                })),
                Cell::new(Line::from(vec![
                    Span::styled(
                        "󰗀 Feed src  ",
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::from(feed.url.as_str()),
                ])),
            ]),
            Row::new([
                Cell::new(Span::styled(
                    " Generator",
                    Style::default().add_modifier(Modifier::BOLD),
                )),
                Cell::new(Span::from(
                    feed.generator.as_deref().unwrap_or(ui::UNKNOWN_SYMBOL),
                )),
                Cell::new(Line::from(vec![
                    Span::styled(
                        "󰈙 Feed type ",
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::from(match feed.r#type {
                        Some(FeedType::RSS0) => "RSS 0",
                        Some(FeedType::RSS1) => "RSS 1",
                        Some(FeedType::RSS2) => "RSS 2",
                        Some(FeedType::Atom) => "Atom",
                        Some(FeedType::JSON) => "JSON Feed",
                        None => ui::UNKNOWN_SYMBOL,
                    }),
                ])),
            ]),
        ];

        let table = Table::new(meta_rows, widths)
            .column_spacing(2)
            .style(cx.theme.subscription.background);
        Widget::render(table, meta_area, buf);

        let entry = |entry: &EntryMeta| {
            let title = entry.title.as_deref().unwrap_or(ui::UNKNOWN_SYMBOL);
            let published = entry
                .published
                .as_ref()
                .or(entry.updated.as_ref())
                .map_or_else(|| ui::UNKNOWN_SYMBOL.to_string(), TimeExt::local_ymd);
            let summary = entry.summary_text(100).unwrap_or(ui::UNKNOWN_SYMBOL.into());

            Row::new([
                Cell::new(Span::from(published)),
                Cell::new(Span::from(title.to_string())),
                Cell::new(Span::from(summary)),
            ])
        };

        let header = Row::new([
            Cell::new(Span::from(" Published")),
            Cell::new(Span::from("󰯂 Entry")),
            Cell::new(Span::from("󱙓 Summary")),
        ]);

        let rows = feed.entries.iter().map(entry);
        let table = Table::new(rows, widths)
            .header(header.style(cx.theme.subscription.header))
            .column_spacing(2)
            .style(cx.theme.subscription.background);

        Widget::render(table, entries_area, buf);
    }
}
