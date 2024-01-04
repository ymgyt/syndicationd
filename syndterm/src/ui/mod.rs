use ratatui::{
    prelude::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::application::State;

pub struct Context<'a> {
    pub app_state: &'a State,
}

pub fn render(frame: &mut Frame, cx: Context<'_>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(frame.size());

    let title = {
        let title_block = Block::default()
            .borders(Borders::ALL)
            .style(Style::default());
        Paragraph::new(Text::styled("Synd", Style::default().fg(Color::Green))).block(title_block)
    };

    let list = {
        let items = if let Some(ref sub) = cx.app_state.user_subscription {
            sub.feeds
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

    let prompt = {
        let text = vec![Span::styled(
            "Subscription | ",
            Style::default().fg(Color::White),
        )];
        Paragraph::new(Line::from(text)).block(Block::default().borders(Borders::ALL))
    };

    frame.render_widget(title, chunks[0]);
    frame.render_widget(list, chunks[1]);
    frame.render_widget(prompt, chunks[2]);
}

/// Create centered Rect
#[allow(dead_code)]
fn centered(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    // get vertically centered rect
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    // then centered horizontally
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(layout[1])[1]
}
