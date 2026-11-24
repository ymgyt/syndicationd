use std::borrow::Cow;

use itertools::Itertools;
use ratatui::{
    prelude::{Alignment, Buffer, Constraint, Layout, Rect},
    style::{Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Cell, Padding, Paragraph, Row, Shadow, Table as RatatuiTable,
        Tabs as RatatuiTabs, Widget,
    },
};
use synd_client::payload::SubscriptionPayload;
use synd_feed::types::{FeedType, FeedUrl};

use crate::{
    application::Populate,
    command::{ListCommand, UnsubscribeTarget},
    types::{self, Feed, TimeExt},
    ui::{
        self, Context,
        extension::RectExt,
        icon,
        widgets::{
            collections::FilterableVec,
            filter::FeedFilterer,
            scrollbar::Scrollbar,
            table::{self, Table},
        },
    },
};

pub struct SubscriptionWidget {
    feeds: FilterableVec<types::Feed, FeedFilterer>,
}

impl SubscriptionWidget {
    pub(crate) fn new() -> Self {
        Self {
            feeds: FilterableVec::new(),
        }
    }

    pub(crate) fn has_subscription(&self) -> bool {
        !self.feeds.is_empty()
    }

    pub(crate) fn is_already_subscribed(&self, url: &FeedUrl) -> bool {
        self.feeds.iter().any(|feed| &feed.url == url)
    }

    pub(crate) fn selected_feed(&self) -> Option<&types::Feed> {
        self.feeds.selected()
    }

    pub(crate) fn update_subscription(
        &mut self,
        populate: Populate,
        subscription: SubscriptionPayload,
    ) {
        let feeds = subscription
            .feeds
            .nodes
            .into_iter()
            .map(types::Feed::from)
            .collect();
        FilterableVec::update(&mut self.feeds, populate, feeds);
    }

    pub(crate) fn update_filterer(&mut self, filterer: FeedFilterer) {
        self.feeds.update_filter(filterer);
    }

    pub(crate) fn remove_unsubscribed_feed(&mut self, url: &FeedUrl) {
        self.feeds.retain(|feed| &feed.url != url);
    }

    pub(crate) fn apply_list(&mut self, command: ListCommand) {
        self.feeds.apply_list(command);
    }
}

impl SubscriptionWidget {
    pub fn render(&self, area: Rect, buf: &mut Buffer, cx: &Context<'_>) {
        let vertical = Layout::vertical([Constraint::Fill(2), Constraint::Fill(1)]);
        let [feeds_area, feed_detail_area] = vertical.areas(area);

        self.render_feeds(feeds_area, buf, cx);
        self.render_feed_detail(feed_detail_area, buf, cx);
    }

    fn render_feeds(&self, area: Rect, buf: &mut Buffer, cx: &Context<'_>) {
        let feeds_area = Table::inner_area(area);

        let (header, widths, rows) = self.feed_rows(cx);

        Table::builder()
            .header(header)
            .widths(widths)
            .rows(rows)
            .selected_idx(self.feeds.selected_index())
            .build(cx)
            .render(feeds_area, buf);

        let header_rows = 2;
        #[allow(clippy::cast_possible_truncation)]
        let scrollbar_area = Rect {
            y: area.y + header_rows,
            height: area
                .height
                .saturating_sub(header_rows)
                .min(self.feeds.len() as u16),
            ..area
        };

        Scrollbar {
            content_length: self.feeds.len(),
            position: self.feeds.selected_index(),
        }
        .render(scrollbar_area, buf, cx);
    }

    fn feed_rows<'a>(
        &'a self,
        cx: &'a Context<'_>,
    ) -> (
        Row<'a>,
        impl IntoIterator<Item = Constraint>,
        impl IntoIterator<Item = Row<'a>>,
    ) {
        let (n, m) = {
            if self.feeds.is_empty() {
                (Cow::Borrowed("-"), Cow::Borrowed("-"))
            } else {
                (
                    Cow::Owned((self.feeds.selected_index() + 1).to_string()),
                    Cow::Owned(self.feeds.len().to_string()),
                )
            }
        };
        let header = Row::new([
            Cell::from(format!("Feed {n}/{m}")),
            Cell::from("URL"),
            Cell::from("Description"),
            Cell::from(Line::from("Updated").right_aligned()),
        ]);

        let constraints = [
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Fill(2),
            Constraint::Length(10),
        ];

        let row = |(feed_meta, selected): (&'a Feed, bool)| {
            let title = feed_meta.title.as_deref().unwrap_or(ui::UNKNOWN_SYMBOL);
            let website_url = feed_meta
                .website_url
                .as_deref()
                .unwrap_or(ui::UNKNOWN_SYMBOL);
            let desc = feed_meta.description.as_deref().unwrap_or("");
            let updated = feed_meta
                .updated_at()
                .map_or_else(|| ui::UNKNOWN_SYMBOL.to_owned(), TimeExt::local_ymd);
            let category = feed_meta.category();
            let icon = cx
                .categories
                .icon(category)
                .unwrap_or_else(|| ui::default_icon());

            Row::new([
                table::title_cell(
                    Span::from(icon.symbol()).fg(icon.color().unwrap_or(cx.theme.default_icon_fg)),
                    title,
                    selected,
                    cx,
                ),
                Cell::from(Span::from(
                    website_url
                        .trim_start_matches("http://")
                        .trim_start_matches("https://")
                        .trim_end_matches('/'),
                )),
                Cell::from(Span::from(desc)),
                Cell::from(Line::from(updated).right_aligned()),
            ])
        };

        (
            header,
            constraints,
            self.feeds.iter_with_selection().map(row),
        )
    }

    #[allow(clippy::too_many_lines)]
    fn render_feed_detail(&self, area: Rect, buf: &mut Buffer, cx: &Context<'_>) {
        let block = Block::new()
            .padding(Padding {
                left: 2,
                right: 1,
                top: 0,
                bottom: 0,
            })
            .borders(Borders::TOP)
            .border_type(BorderType::Plain);

        let inner = block.inner(area);
        Widget::render(block, area, buf);

        let Some(feed) = self.selected_feed() else {
            return;
        };

        let vertical = Layout::vertical([Constraint::Length(4), Constraint::Min(0)]);
        let [meta_area, entries_area] = vertical.areas(inner);
        let entries_area = Block::new().padding(Padding::top(1)).inner(entries_area);

        let meta_widths = [
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
                    Span::styled("󰗀 Src  ", Style::default().add_modifier(Modifier::BOLD)),
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
                    Span::styled("󰈙 Type ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::from(match feed.feed_type {
                        Some(FeedType::RSS0) => "RSS 0",
                        Some(FeedType::RSS1) => "RSS 1",
                        Some(FeedType::RSS2) => "RSS 2",
                        Some(FeedType::Atom) => "Atom",
                        Some(FeedType::JSON) => "JSON Feed",
                        None => ui::UNKNOWN_SYMBOL,
                    }),
                ])),
            ]),
            Row::new([
                Cell::new(Span::styled(
                    " Category",
                    Style::default().add_modifier(Modifier::BOLD),
                )),
                Cell::new(Span::from(feed.category().as_str())),
                Cell::new(Line::from(vec![
                    Span::styled(" Req  ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::from(feed.requirement().to_string()),
                ])),
            ]),
            Row::new([
                Cell::new(Span::from(concat!(icon!(calendar), " Updated")).bold()),
                Cell::new(
                    feed.updated_at()
                        .map_or_else(|| ui::UNKNOWN_SYMBOL.to_owned(), TimeExt::local_ymd_hm),
                )
                .column_span(2),
            ]),
        ];

        let table = RatatuiTable::new(meta_rows, meta_widths)
            .column_spacing(1)
            .style(cx.theme.subscription.background);
        Widget::render(table, meta_area, buf);

        let rows = feed.entries.iter().map(|entry| {
            let title = entry.title.as_deref().unwrap_or(ui::UNKNOWN_SYMBOL);
            let excerpt = entry
                .excerpt
                .as_ref()
                .map_or(ui::UNKNOWN_SYMBOL, |excerpt| excerpt.text.as_str());

            Row::new([
                Cell::new(Span::from(title)),
                Cell::new(Span::from(excerpt)),
                table::date_cell(entry.published.as_ref().or(entry.updated.as_ref())),
            ])
        });

        let header = Row::new([
            Cell::new(Span::from("Entry")),
            Cell::new(Span::from("Excerpt")),
            Cell::new(Line::from("Date").right_aligned()),
        ]);

        let table = RatatuiTable::new(
            rows,
            [Constraint::Fill(1), Constraint::Fill(2), table::DATE_WIDTH],
        )
        .header(header.style(cx.theme.subscription.header))
        .column_spacing(1)
        .style(cx.theme.subscription.background);

        Widget::render(table, entries_area, buf);
    }
}

pub(crate) fn render_unsubscribe(
    area: Rect,
    buf: &mut Buffer,
    cx: &Context<'_>,
    feed: &UnsubscribeTarget,
    yes_selected: bool,
) {
    let area = {
        let area = RectExt::centered(area, 60, 60);
        let vertical = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Min(12),
            Constraint::Fill(2),
        ]);
        let [_, area, _] = vertical.areas(area);
        area.reset(buf);
        area
    };

    let block = Block::new()
        .title_top("Unsubscribe")
        .title_alignment(Alignment::Center)
        .title_style(Style::new().add_modifier(Modifier::BOLD))
        .padding(Padding {
            left: 1,
            right: 1,
            top: 1,
            bottom: 1,
        })
        .borders(Borders::ALL)
        .shadow(Shadow::dark_shade().style(cx.theme.base.add_modifier(Modifier::DIM)))
        .style(cx.theme.base);

    let inner_area = block.inner(area);
    let vertical = Layout::vertical([Constraint::Length(6), Constraint::Fill(1)]);
    let [info_area, selection_area] = vertical.areas(inner_area);

    block.render(area, buf);

    // for align line
    let feed_n = "Feed: ".len() + feed.title.as_deref().unwrap_or("-").len();
    let url_n = "URL : ".len() + feed.url.as_str().len();

    Paragraph::new(vec![
        Line::from("Do you unsubscribe from this feed?"),
        Line::from(""),
        Line::from(vec![
            Span::from("Feed: "),
            Span::from(feed.title.as_deref().unwrap_or("-")).bold(),
            Span::from(" ".repeat(url_n.saturating_sub(feed_n))),
        ]),
        Line::from(vec![
            Span::from("URL : "),
            Span::from(feed.url.to_string()).bold(),
            Span::from(" ".repeat(feed_n.saturating_sub(url_n))),
        ]),
    ])
    .alignment(Alignment::Center)
    .block(
        Block::new()
            .borders(Borders::BOTTOM)
            .border_type(BorderType::Plain)
            .border_style(Style::new().add_modifier(Modifier::DIM)),
    )
    .render(info_area, buf);

    // align center
    let horizontal =
        Layout::horizontal([Constraint::Fill(1), Constraint::Min(1), Constraint::Fill(1)]);
    let [_, selection_area, _] = horizontal.areas(selection_area);

    RatatuiTabs::new([" Yes ", " No "])
        .style(cx.theme.tabs)
        .divider("")
        .padding("  ", "  ")
        .select(usize::from(!yes_selected))
        .highlight_style(cx.theme.selection_popup.highlight)
        .render(selection_area, buf);
}

#[cfg(test)]
mod tests {
    use fake::{Fake as _, Faker};
    use synd_feed::types::Requirement;

    use super::*;

    fn feed(url: &str) -> types::Feed {
        let feed: types::Feed = Faker.fake();
        feed.with_url(url.try_into().unwrap())
            .with_requirement(Requirement::May)
    }

    #[test]
    fn move_last_selects_last_feed() {
        let mut subscription = SubscriptionWidget::new();
        subscription.feeds.update(
            Populate::Replace,
            vec![
                feed("https://example.com/1.xml"),
                feed("https://example.com/2.xml"),
                feed("https://example.com/3.xml"),
            ],
        );

        subscription.apply_list(ListCommand::Last);

        assert_eq!(subscription.feeds.selected_index(), 2);
        assert_eq!(
            subscription.selected_feed().map(|feed| feed.url.as_str()),
            Some("https://example.com/3.xml")
        );
    }
}
