use std::borrow::Cow;

use itertools::Itertools;
use ratatui::{
    prelude::{Alignment, Buffer, Constraint, Layout, Rect},
    style::{Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Cell, HighlightSpacing, Padding, Paragraph, Row,
        StatefulWidget, Table, TableState, Tabs, Widget,
    },
};
use synd_feed::types::{FeedType, FeedUrl};

use crate::{
    application::{Direction, Populate},
    client::query::subscription::SubscriptionOutput,
    types::{self, EntryMeta, Feed, RequirementExt, TimeExt},
    ui::{
        self,
        components::{collections::FilterableVec, filter::FeedFilterer},
        extension::RectExt,
        widgets::scrollbar::Scrollbar,
        Context,
    },
};

pub struct Subscription {
    feeds: FilterableVec<types::Feed, FeedFilterer>,

    unsubscribe_popup: UnsubscribePopup,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum UnsubscribeSelection {
    Yes,
    No,
}

impl UnsubscribeSelection {
    fn toggle(self) -> Self {
        match self {
            UnsubscribeSelection::Yes => UnsubscribeSelection::No,
            UnsubscribeSelection::No => UnsubscribeSelection::Yes,
        }
    }
}

struct UnsubscribePopup {
    selection: UnsubscribeSelection,
    selected_feed: Option<types::Feed>,
}

impl Subscription {
    pub(crate) fn new() -> Self {
        Self {
            feeds: FilterableVec::new(),
            unsubscribe_popup: UnsubscribePopup {
                selection: UnsubscribeSelection::Yes,
                selected_feed: None,
            },
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

    pub(crate) fn show_unsubscribe_popup(&mut self, show: bool) {
        if show {
            self.unsubscribe_popup.selected_feed = self.selected_feed().cloned();
        } else {
            self.unsubscribe_popup.selected_feed = None;
        }
    }

    pub(crate) fn unsubscribe_popup_selection(
        &self,
    ) -> (UnsubscribeSelection, Option<&types::Feed>) {
        (
            self.unsubscribe_popup.selection,
            self.unsubscribe_popup.selected_feed.as_ref(),
        )
    }

    pub(crate) fn update_subscription(
        &mut self,
        populate: Populate,
        subscription: SubscriptionOutput,
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

    pub(crate) fn upsert_subscribed_feed(&mut self, feed: types::Feed) {
        let url = feed.url.clone();
        self.feeds.upsert_first(feed, |x| x.url == url);
    }

    pub(crate) fn remove_unsubscribed_feed(&mut self, url: &FeedUrl) {
        self.feeds.retain(|feed| &feed.url != url);
    }

    pub(crate) fn move_selection(&mut self, direction: Direction) {
        self.feeds.move_selection(direction);
    }

    pub(crate) fn move_first(&mut self) {
        self.feeds.move_first();
    }

    pub(crate) fn move_last(&mut self) {
        self.feeds.move_first();
    }

    pub(crate) fn move_unsubscribe_popup_selection(&mut self, direction: Direction) {
        if matches!(direction, Direction::Left | Direction::Right) {
            self.unsubscribe_popup.selection = self.unsubscribe_popup.selection.toggle();
        }
    }
}

impl Subscription {
    pub fn render(&self, area: Rect, buf: &mut Buffer, cx: &Context<'_>) {
        let vertical = Layout::vertical([Constraint::Fill(2), Constraint::Fill(1)]);
        let [feeds_area, feed_detail_area] = vertical.areas(area);

        self.render_feeds(feeds_area, buf, cx);
        self.render_feed_detail(feed_detail_area, buf, cx);

        if let Some(feed) = self.unsubscribe_popup.selected_feed.as_ref() {
            self.render_unsubscribe_popup(area, buf, cx, feed);
        }
    }

    fn render_feeds(&self, area: Rect, buf: &mut Buffer, cx: &Context<'_>) {
        let feeds_area = Block::new().padding(Padding::top(1)).inner(area);

        let mut feeds_state = TableState::new()
            .with_offset(0)
            .with_selected(self.feeds.selected_index());

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
            .highlight_style(
                cx.theme
                    .subscription
                    .selected_feed
                    .add_modifier(cx.table_highlight_modifier()),
            )
            .highlight_spacing(HighlightSpacing::WhenSelected);

        StatefulWidget::render(feeds, feeds_area, buf, &mut feeds_state);

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
            Cell::from("Updated"),
            Cell::from(format!("Feed {n}/{m}")),
            Cell::from("URL"),
            Cell::from("Description"),
            Cell::from("Req"),
        ]);

        let constraints = [
            Constraint::Length(10),
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Fill(2),
            Constraint::Length(4),
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
            let website_url = feed_meta
                .website_url
                .as_deref()
                .unwrap_or(ui::UNKNOWN_SYMBOL);
            let desc = feed_meta.description.as_deref().unwrap_or("");
            let requirement = feed_meta.requirement().label(&cx.theme.requirement);
            let category = feed_meta.category();
            let icon = cx
                .categories
                .icon(category)
                .unwrap_or_else(|| ui::default_icon());

            Row::new([
                Cell::from(Span::from(updated)),
                Cell::from(Line::from(vec![
                    Span::from(icon.symbol()).fg(icon.color().unwrap_or(cx.theme.default_icon_fg)),
                    Span::from(" "),
                    Span::from(title),
                ])),
                Cell::from(Span::from(
                    website_url
                        .trim_start_matches("http://")
                        .trim_start_matches("https://")
                        .trim_end_matches('/'),
                )),
                Cell::from(Span::from(desc)),
                Cell::from(Line::from(vec![requirement, Span::from(" ")])),
            ])
        };

        (header, constraints, self.feeds.iter().map(row))
    }

    #[allow(clippy::too_many_lines)]
    fn render_feed_detail(&self, area: Rect, buf: &mut Buffer, cx: &Context<'_>) {
        let block = Block::new()
            .padding(Padding {
                left: 2,
                right: 2,
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

        let vertical = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]);
        let [meta_area, entries_area] = vertical.areas(inner);
        let entries_area = Block::new().padding(Padding::top(1)).inner(entries_area);

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
        ];

        let table = Table::new(meta_rows, widths)
            .column_spacing(1)
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
            Cell::new(Span::from("Published")),
            Cell::new(Span::from("Entry")),
            Cell::new(Span::from("Summary")),
        ]);

        let rows = feed.entries.iter().map(entry);
        let table = Table::new(rows, widths)
            .header(header.style(cx.theme.subscription.header))
            .column_spacing(1)
            .style(cx.theme.subscription.background);

        Widget::render(table, entries_area, buf);
    }

    fn render_unsubscribe_popup(
        &self,
        area: Rect,
        buf: &mut Buffer,
        cx: &Context<'_>,
        feed: &types::Feed,
    ) {
        let area = {
            let area = area.centered(60, 60);
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

        Tabs::new([" Yes ", " No "])
            .style(cx.theme.tabs)
            .divider("")
            .padding("  ", "  ")
            .select(match self.unsubscribe_popup.selection {
                UnsubscribeSelection::Yes => 0,
                UnsubscribeSelection::No => 1,
            })
            .highlight_style(cx.theme.selection_popup.highlight)
            .render(selection_area, buf);
    }
}
