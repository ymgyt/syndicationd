use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Constraint, Layout, Margin, Rect},
    style::{Style, Stylize as _},
    text::Line,
    widgets::Widget as _,
};

use super::{Draft, Field, RenderContext, SubscribeForm};
use crate::ui::widgets::prompt::{Prompt, RenderCursor};

/// One logical row, in reading and keyboard focus order.
pub(super) enum Row {
    Heading(&'static str),
    Space,
    Field(Field),
    Label([Field; 4]),
}

impl Row {
    pub(super) fn fields(&self) -> &[Field] {
        match self {
            Self::Heading(_) | Self::Space => &[],
            Self::Field(field) => std::slice::from_ref(field),
            Self::Label(fields) => fields,
        }
    }
}

impl Draft {
    pub(super) fn rows(&self) -> Vec<Row> {
        let mut rows = vec![
            Row::Heading("Subscribe"),
            Row::Space,
            Row::Heading("Feed URL (required)"),
            Row::Field(Field::Url),
            Row::Space,
            Row::Heading("Labels"),
            Row::Field(Field::Category),
        ];
        rows.extend((0..self.labels.len()).map(|i| {
            Row::Label([
                Field::Key(i),
                Field::Value(i),
                Field::Format(i),
                Field::Remove(i),
            ])
        }));
        rows.extend([
            Row::Field(Field::Add),
            Row::Space,
            Row::Field(Field::PollEvery),
            Row::Space,
            Row::Field(Field::Submit),
        ]);
        rows
    }
}

impl SubscribeForm {
    pub(super) fn render_form(&self, frame: &mut Frame<'_>, cx: &RenderContext<'_>) {
        let body = cx.frame.full_screen_content().inner(Margin::new(1, 0));
        let rows = self.draft.rows();
        let rows = if body.width < 48 {
            let mut stacked = Vec::new();
            for row in rows {
                match row {
                    Row::Label(fields) => stacked.extend(fields.map(Row::Field)),
                    row => stacked.push(row),
                }
            }
            stacked
        } else {
            rows
        };
        let active = rows
            .iter()
            .position(|row| row.fields().contains(&self.focus))
            .unwrap_or_default();
        let first = (active + 1).saturating_sub(usize::from(body.height));
        let accent = cx.ui.theme.application_title;
        let buf = frame.buffer_mut();
        for (row, area) in rows[first..].iter().zip(body.rows()) {
            let [marker, content] =
                Layout::horizontal([Constraint::Length(2), Constraint::Fill(1)]).areas(area);
            if row.fields().contains(&self.focus) {
                Line::styled("›", accent).render(marker, buf);
            }
            match row {
                Row::Heading(title) => Line::styled(*title, accent).render(content, buf),
                Row::Space => {}
                Row::Field(field) => self.render_field(*field, content, buf, accent),
                Row::Label(fields) => {
                    let columns: [Rect; 4] = Layout::horizontal([
                        Constraint::Fill(1),
                        Constraint::Fill(2),
                        Constraint::Length(8),
                        Constraint::Length(10),
                    ])
                    .spacing(1)
                    .areas(content);
                    for (&field, column) in fields.iter().zip(columns) {
                        self.render_field(field, column, buf, accent);
                    }
                }
            }
        }
    }

    fn render_field(&self, field: Field, area: Rect, buf: &mut Buffer, accent: Style) {
        let draft = &self.draft;
        let selected = self.focus == field;
        let control = match field {
            Field::Url => Control::Input("", &draft.url, "https://…"),
            Field::Category => Control::Input("category", &draft.category, "unset"),
            Field::Key(i) => Control::Input("", &draft.labels[i].key, "key"),
            Field::Value(i) => Control::Input("", &draft.labels[i].value, "value"),
            Field::PollEvery => {
                Control::Input("Poll every", &draft.poll_every, "default  (e.g. 30m, 2h)")
            }
            Field::Format(i) => Control::Button(if draft.labels[i].json {
                "[ JSON ]"
            } else {
                "[ Text ]"
            }),
            Field::Remove(_) => Control::Button("[ Remove ]"),
            Field::Add => Control::Button("[+ Add label]"),
            Field::Submit => Control::Button("[ Subscribe ]"),
        };
        match control {
            Control::Input(caption, prompt, hint) => {
                text_input(caption, prompt, hint, area, buf, selected);
            }
            Control::Button(text) => {
                let style = if selected {
                    accent.reversed()
                } else {
                    Style::default()
                };
                Line::styled(text, style).render(area, buf);
            }
        }
    }
}

enum Control<'a> {
    Input(&'static str, &'a Prompt, &'static str),
    Button(&'static str),
}

fn text_input(
    caption: &str,
    prompt: &Prompt,
    hint: &str,
    area: Rect,
    buf: &mut Buffer,
    selected: bool,
) {
    let area = if caption.is_empty() {
        area
    } else {
        let [label, value] = Layout::horizontal([
            Constraint::Length(14.min(area.width / 2)),
            Constraint::Fill(1),
        ])
        .areas(area);
        Line::raw(caption).render(label, buf);
        value
    };
    if prompt.line().is_empty() {
        Line::raw(hint).dim().render(area, buf);
        if selected && !area.is_empty() {
            buf[(area.x, area.y)].set_style(Style::default().reversed());
        }
    } else {
        prompt.render(
            area,
            buf,
            if selected {
                RenderCursor::Enable
            } else {
                RenderCursor::Disable
            },
        );
    }
}
