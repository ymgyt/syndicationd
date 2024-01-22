use ratatui::{
    prelude::{Buffer, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{List, ListItem, Widget},
};

use crate::{client::query::subscription::SubscriptionOutput, types, ui::Context};

pub struct Subscription {
    feed_metas: Vec<types::FeedMeta>,
}

impl Subscription {
    pub fn new() -> Self {
        Self {
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
        self.feed_metas.push(feed)
    }
}

impl Subscription {
    pub fn render(&self, area: Rect, buf: &mut Buffer, _cx: &Context<'_>) {
        let list = {
            let items = self
                .feed_metas
                .iter()
                .map(|feed| {
                    tracing::info!("{feed:?}");
                    Line::from(vec![
                        Span::styled(feed.title.as_deref().unwrap_or("???"), Style::default()),
                        Span::styled(
                            format!(
                                " | {}",
                                feed.updated
                                    .as_ref()
                                    .map(|t| t.naive_local().to_string())
                                    .unwrap_or("???".into())
                            ),
                            Style::default(),
                        ),
                        Span::styled(
                            format!(" | {}", feed.website_url.as_deref().unwrap_or("???")),
                            Style::default(),
                        ),
                    ])
                })
                .map(ListItem::new);

            List::new(items)
        };

        list.render(area, buf);
    }
}
