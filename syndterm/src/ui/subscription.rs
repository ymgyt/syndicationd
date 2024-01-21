use ratatui::{
    prelude::{Buffer, Margin, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Clear, List, ListItem, Widget},
};

use crate::{client::query::subscription::SubscriptionOutput, types::FeedMeta, ui::Context};

pub struct Subscription {
    feed_metas: Vec<FeedMeta>,
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
            .map(|node| FeedMeta::new(node.title, node.url));
        self.feed_metas = feed_metas.collect();
    }

    pub fn add_new_feed(&mut self, feed: FeedMeta) {
        self.feed_metas.push(feed)
    }
}

impl Subscription {
    pub fn render(&self, area: Rect, buf: &mut Buffer, _cx: &Context<'_>) {
        let area = area.inner(&Margin {
            vertical: 1,
            horizontal: 2,
        });
        Clear.render(area, buf);
        let list = {
            let items = self
                .feed_metas
                .iter()
                .map(|feed| {
                    Line::from(vec![
                        Span::styled(format!("Title: {}", &feed.title()), Style::default()),
                        Span::styled(format!("Url: {}", &feed.url()), Style::default()),
                    ])
                })
                .map(ListItem::new);

            List::new(items)
        };

        list.render(area, buf);
    }
}
