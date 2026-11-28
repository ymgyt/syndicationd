use crate::command::SubscriptionMode;
use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Paragraph, Widget as _, Wrap},
};
use synd_feed::feed::DEFAULT_POLLING_INTERVAL;

use super::{Field, FormState, RenderContext, SubscriptionForm, draft::interval_text};
use crate::ui::{
    theme::Theme,
    widgets::{
        Button,
        prompt::{Prompt, RenderCursor},
    },
};

/// A row in the scrollable body. The title and submit action stay on screen.
enum Row<'a> {
    Space,
    Url,
    Labels,
    Control(&'a str, Field),
    Label(usize),
    LabelActions(usize),
    Error(&'a str),
}

impl Row<'_> {
    fn controls(&self) -> impl Iterator<Item = Field> {
        match self {
            Self::Control(_, field) => [Some(*field), None],
            Self::Label(i) => [Some(Field::Label(*i)), Some(Field::Remove(*i))],
            Self::LabelActions(i) => [Some(Field::Done(*i)), Some(Field::Remove(*i))],
            _ => [None, None],
        }
        .into_iter()
        .flatten()
    }

    fn contains(&self, field: Field) -> bool {
        self.controls().any(|control| control == field)
            || matches!((self, field), (Self::Label(i), Field::Key(j) | Field::Value(j)) if *i == j)
    }

    fn height(&self, width: u16) -> u16 {
        let Self::Error(message) = self else {
            return 1;
        };
        u16::try_from(
            Paragraph::new(*message)
                .wrap(Wrap { trim: false })
                .line_count(width),
        )
        .unwrap_or(u16::MAX)
        .max(1)
    }
}

impl SubscriptionForm {
    pub(super) fn focus_order(&self) -> Vec<Field> {
        self.rows()
            .iter()
            .flat_map(Row::controls)
            .chain([Field::Submit])
            .collect()
    }

    fn rows(&self) -> Vec<Row<'_>> {
        let mut rows = vec![
            if self.mode == SubscriptionMode::Edit {
                Row::Url
            } else {
                Row::Control("Feed URL", Field::Url)
            },
            Row::Space,
            Row::Control("Poll every", Field::PollEvery),
            Row::Space,
            Row::Labels,
        ];
        let expanded = self.focus.expanded_label();
        for i in 0..self.draft.labels.len() {
            if expanded == Some(i) {
                rows.extend([
                    Row::Control("Key", Field::Key(i)),
                    Row::Control("Value", Field::Value(i)),
                    Row::LabelActions(i),
                ]);
            } else {
                rows.push(Row::Label(i));
            }
        }
        rows.push(Row::Control("", Field::Add));
        if let FormState::Invalid(field, message) = &self.state {
            let position = rows.iter().position(|row| row.contains(*field));
            if let Some(i) = position {
                rows.insert(i + 1, Row::Error(message));
            } else if *field == Field::Submit {
                rows.extend([Row::Space, Row::Error(message)]);
            }
        }
        rows
    }

    pub(super) fn render_form(&self, frame: &mut Frame<'_>, cx: &RenderContext<'_>) {
        let dialog = cx.frame.dialog();
        let inner_width = dialog.inner_width();
        let rows = self.rows();
        let heights: Vec<_> = rows.iter().map(|row| row.height(inner_width)).collect();
        let lines = heights.iter().copied().fold(0u16, u16::saturating_add);
        let theme = cx.ui.theme;
        let buf = frame.buffer_mut();
        let title = match self.mode {
            SubscriptionMode::Create => "Subscribe to feed",
            SubscriptionMode::Edit => "Edit subscription",
        };
        let (body, footer) = dialog.render(buf, theme, title, lines, 13);
        self.render_body(body, buf, theme, &rows, &heights);
        self.render_field(Field::Submit, footer, buf, theme);
    }

    fn render_body(
        &self,
        body: Rect,
        buf: &mut Buffer,
        theme: &Theme,
        rows: &[Row<'_>],
        heights: &[u16],
    ) {
        let active = rows
            .iter()
            .position(|row| row.contains(self.focus))
            .unwrap_or(rows.len() - 1);
        let mut first = active;
        let mut used = heights[active];
        if matches!(rows.get(active + 1), Some(Row::Error(_))) {
            used = used.saturating_add(heights[active + 1]);
        }
        while first > 0 && used.saturating_add(heights[first - 1]) <= body.height {
            first -= 1;
            used += heights[first];
        }
        let mut y = body.y;
        for (row, height) in rows[first..].iter().zip(&heights[first..]) {
            if y >= body.bottom() {
                break;
            }
            let area = Rect::new(body.x, y, body.width, (*height).min(body.bottom() - y));
            match row {
                Row::Labels => {
                    Line::styled("Labels", theme.application_title).render(area, buf);
                }
                Row::Space => {}
                Row::Url => {
                    let value = captioned(area, "Feed URL", buf);
                    Line::raw(self.draft.url.line()).render(value, buf);
                }
                Row::Error(message) => {
                    Paragraph::new(*message)
                        .style(theme.error.message)
                        .wrap(Wrap { trim: false })
                        .render(area, buf);
                }
                Row::Control(caption, field) => {
                    let mut value = captioned(area, caption, buf);
                    if *field == Field::PollEvery {
                        value.width = value.width.min(14);
                    }
                    self.render_field(*field, value, buf, theme);
                }
                Row::Label(i) => {
                    let area = captioned(area, self.draft.labels[*i].key.line(), buf);
                    let [mut value, remove] =
                        Layout::horizontal([Constraint::Fill(1), Constraint::Length(6)])
                            .spacing(1)
                            .areas(area);
                    value.width = value.width.min(26);
                    self.render_field(Field::Label(*i), value, buf, theme);
                    self.render_field(Field::Remove(*i), remove, buf, theme);
                }
                Row::LabelActions(i) => {
                    let area = captioned(area, "", buf);
                    let [done, remove] =
                        Layout::horizontal([Constraint::Fill(1), Constraint::Length(6)])
                            .spacing(1)
                            .areas(area);
                    self.render_field(Field::Done(*i), done, buf, theme);
                    self.render_field(Field::Remove(*i), remove, buf, theme);
                }
            }
            y += area.height;
        }
    }

    fn render_field(&self, field: Field, area: Rect, buf: &mut Buffer, theme: &Theme) {
        let selected = self.focus == field && !matches!(self.state, FormState::Submitting(_));
        let inline = |label| Line::from(Button::new(label, selected, theme).inline());
        let prompt: &Prompt = match field {
            Field::Url => &self.draft.url,
            Field::Label(i) | Field::Value(i) => &self.draft.labels[i].value,
            Field::Key(i) => &self.draft.labels[i].key,
            Field::PollEvery => &self.draft.poll_every,
            Field::Done(_) => return inline("Done").render(area, buf),
            Field::Remove(_) => return inline("Remove").render(area, buf),
            Field::Add => return inline("+ Add label").render(area, buf),
            Field::Submit => {
                let (action, pending) = match self.mode {
                    SubscriptionMode::Create => ("Subscribe", "Subscribing…"),
                    SubscriptionMode::Edit => ("Save", "Saving…"),
                };
                let line = if matches!(self.state, FormState::Submitting(_)) {
                    Line::from(Span::styled(pending, theme.tabs_selected))
                } else {
                    Button::new(action, selected, theme).into()
                };
                return line.right_aligned().render(area, buf);
            }
        };
        buf.set_style(area, theme.input_surface());
        if field == Field::PollEvery && prompt.line().is_empty() {
            let default = interval_text(DEFAULT_POLLING_INTERVAL.as_secs());
            Line::from(Span::styled(default, theme.input_placeholder())).render(area, buf);
            if selected && !area.is_empty() {
                buf[(area.x, area.y)].set_style(Style::new().reversed());
            }
            return;
        }
        let cursor = if selected {
            RenderCursor::Enable
        } else {
            RenderCursor::Disable
        };
        prompt.render(area, buf, cursor);
    }
}

/// Aligns values, including continuation rows with no caption.
fn captioned(area: Rect, caption: &str, buf: &mut Buffer) -> Rect {
    let [label, value] = Layout::horizontal([
        Constraint::Length(14.min(area.width / 2).saturating_sub(1)),
        Constraint::Fill(1),
    ])
    .spacing(1)
    .areas(area);
    Line::raw(caption).render(label, buf);
    value
}
