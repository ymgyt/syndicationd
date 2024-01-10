use ratatui::{
    prelude::{Buffer, Margin, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Clear, List, ListItem, Widget},
};

use crate::{
    client::query::subscription::{SubscriptionOutput, SubscriptionOutputFeedsNodes},
    ui::Context,
};

pub struct Subscription {
    subscription: Option<SubscriptionOutput>,
}

impl Subscription {
    pub fn new() -> Self {
        Self { subscription: None }
    }

    pub fn has_subscription(&self) -> bool {
        self.subscription.is_some()
    }

    pub fn update_subscription(&mut self, subscription: SubscriptionOutput) {
        self.subscription = Some(subscription);
    }

    pub fn add_new_feed(&mut self, url: String) {
        let Some(sub) = self.subscription.as_mut() else {
            return;
        };
        sub.feeds.nodes.push(SubscriptionOutputFeedsNodes { url });
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
            let items = if let Some(ref sub) = self.subscription {
                sub.feeds
                    .nodes
                    .iter()
                    .map(|feed| feed.url.as_str())
                    .map(|url| {
                        Line::from(Span::styled(
                            format!("Url: {url}"),
                            Style::default().fg(Color::Green),
                        ))
                    })
                    .map(ListItem::new)
                    .collect()
            } else {
                vec![]
            };

            List::new(items)
        };

        list.render(area, buf);
    }
}
