use ratatui::{
    prelude::{Buffer, Constraint, Margin, Rect},
    text::Span,
    widgets::{Cell, HighlightSpacing, Row, StatefulWidget, Table, TableState},
};

use crate::{
    application::Direction,
    client::query::subscription::SubscriptionOutput,
    types::{self, FeedMeta},
    ui::{self, Context},
};

pub struct Subscription {
    selected_feed_meta_index: usize,
    feed_metas: Vec<types::FeedMeta>,
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

    pub fn update_subscription(&mut self, subscription: SubscriptionOutput) {
        let feed_metas = subscription
            .feeds
            .nodes
            .into_iter()
            .map(types::FeedMeta::from);
        self.feed_metas = feed_metas.collect();
    }

    pub fn add_new_feed(&mut self, feed: types::FeedMeta) {
        self.feed_metas.insert(0, feed)
    }

    pub fn move_selection(&mut self, direction: Direction) {
        self.selected_feed_meta_index =
            direction.apply(self.selected_feed_meta_index, self.feed_metas.len(), false);
    }
}

impl Subscription {
    pub fn render(&self, area: Rect, buf: &mut Buffer, cx: &Context<'_>) {
        // padding
        let area = area.inner(&Margin {
            vertical: 1,
            horizontal: 1,
        });

        let mut feeds_state = TableState::new()
            .with_offset(0)
            .with_selected(self.selected_feed_meta_index);

        let (header, widths, rows) = self.rows(cx);

        let feeds = Table::new(rows, widths)
            .header(header.style(cx.theme.subscription.header))
            .column_spacing(2)
            .style(cx.theme.subscription.background)
            .highlight_symbol(">> ")
            .highlight_style(cx.theme.subscription.selected_feed)
            .highlight_spacing(HighlightSpacing::WhenSelected);

        feeds.render(area, buf, &mut feeds_state);
    }

    fn rows<'a>(
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
            Constraint::Max(20),
            Constraint::Length(10),
            Constraint::Max(40),
            Constraint::Max(100),
        ];

        let row = |feed_meta: &'a FeedMeta| {
            let title = feed_meta.title.as_deref().unwrap_or(ui::UNKNOWN_SYMBOL);
            let updated = feed_meta
                .updated
                .as_ref()
                .map(|t| t.naive_local().format("%Y-%m-%d").to_string())
                .unwrap_or(ui::UNKNOWN_SYMBOL.to_string());
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
}
