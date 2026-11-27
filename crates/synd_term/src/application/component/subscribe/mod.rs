use std::rc::Rc;

use serde_json::Value;
use synd_client::payload::{CrawlPolicyInput, PollingPolicyInput, SubscribeFeedInput};
use synd_feed::types::{Category, FeedUrl, SubscriptionLabels};
use tuiframework::Component;

use super::{ComponentId, RenderContext, Ui};
use crate::{
    application::{
        Direction, RequestError, RequestKind, effects::Effects, input_parser::polling_interval,
        request::synd_api_error_message,
    },
    command::{Command, FeedsCommand, SubscribeCommand},
    config::Categories,
    event::Event,
    keymap::{Layer, LayerStack},
    operation::SubscriptionOrigin,
    ui::widgets::prompt::Prompt,
};

mod render;

/// Owns the draft until the subscription is accepted or the user cancels.
pub(super) struct SubscribeForm {
    categories: Rc<Categories>,
    draft: Draft,
    focus: Field,
    state: FormState,
}

enum FormState {
    Editing,
    Invalid(Field, String),
    Submitting,
}

struct Draft {
    url: Prompt,
    labels: Vec<LabelRow>,
    poll_every: Prompt,
}

impl Default for Draft {
    fn default() -> Self {
        Self {
            url: Prompt::default(),
            labels: vec![LabelRow {
                key: "category".into(),
                ..LabelRow::default()
            }],
            poll_every: Prompt::default(),
        }
    }
}

#[derive(Default)]
struct LabelRow {
    key: Prompt,
    value: Prompt,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Field {
    Url,
    PollEvery,
    /// The compact row edits the value directly. Confirm opens the key and value editor.
    Label(usize),
    Key(usize),
    Value(usize),
    Done(usize),
    Remove(usize),
    Add,
    Submit,
}

impl Field {
    fn expanded_label(self) -> Option<usize> {
        match self {
            Self::Key(i) | Self::Value(i) | Self::Done(i) => Some(i),
            _ => None,
        }
    }

    fn shares_value_with(self, other: Self) -> bool {
        self == other || matches!((self, other), (Self::Label(i), Self::Value(j)) if i == j)
    }
}

impl Component<Ui> for SubscribeForm {
    fn key_layers(&self) -> &[Layer] {
        match self.state {
            FormState::Submitting => &[],
            _ => &[Layer::TextInput, Layer::Subscribe],
        }
    }

    fn mask_lower_layers(&self, layers: &mut LayerStack) {
        layers.truncate_after(Layer::App);
    }

    fn apply_command(&mut self, command: Command) -> Effects {
        match command {
            Command::Subscribe(SubscribeCommand::AlreadySubscribed) => {
                self.show_error(Field::Url, "Already subscribed to this URL".into());
            }
            _ if matches!(self.state, FormState::Submitting) => {}
            Command::Subscribe(SubscribeCommand::Open) => {
                self.draft = Draft::default();
                self.focus = Field::Url;
                self.state = FormState::Editing;
                return Effects::NONE.send(ComponentId::Subscribe, Command::Open);
            }
            Command::Subscribe(SubscribeCommand::Cancel) => {
                return Effects::NONE.send(ComponentId::Subscribe, Command::Close);
            }
            Command::Subscribe(SubscribeCommand::Confirm) => return self.confirm(),
            Command::Subscribe(SubscribeCommand::Move(direction)) => self.move_focus(direction),
            Command::Prompt(action) => {
                if let Some(prompt) = self.draft.prompt_mut(self.focus)
                    && prompt.apply(&action)
                    && matches!(self.state, FormState::Invalid(field, _) if self.focus.shares_value_with(field) || field == Field::Submit)
                {
                    self.state = FormState::Editing;
                }
            }
            _ => {}
        }
        Effects::NONE
    }

    fn subscribes(&self, event: &Event) -> bool {
        matches!(self.state, FormState::Submitting)
            && matches!(
                event,
                Event::RequestCompleted {
                    kind: RequestKind::SubscribeFeed {
                        origin: SubscriptionOrigin::Form,
                        ..
                    },
                    ..
                }
            )
    }

    fn apply_event(&mut self, event: &Event) -> Effects {
        match event {
            Event::RequestCompleted { result: Ok(()), .. } => {
                self.state = FormState::Editing;
                return Effects::NONE.send(ComponentId::Subscribe, Command::Close);
            }
            Event::RequestCompleted {
                result: Err(RequestError::SyndApi(error)),
                ..
            } => {
                tracing::error!(%error, "subscription request failed");
                self.show_error(
                    Field::Submit,
                    format!("Could not subscribe: {}", synd_api_error_message(error)),
                );
            }
            _ => {}
        }
        Effects::NONE
    }

    fn render(&self, frame: &mut ratatui::Frame<'_>, cx: &RenderContext<'_>) {
        self.render_form(frame, cx);
    }
}

impl SubscribeForm {
    pub(super) fn new(categories: Rc<Categories>) -> Self {
        Self {
            categories,
            draft: Draft::default(),
            focus: Field::Url,
            state: FormState::Editing,
        }
    }

    fn move_focus(&mut self, direction: Direction) {
        let fields = self.focus_order();
        let current = fields
            .iter()
            .position(|&field| field == self.focus)
            .expect("focused control");
        self.focus = fields[direction.apply(current, fields.len())];
    }

    fn confirm(&mut self) -> Effects {
        match self.focus {
            Field::Url | Field::Submit => match self.draft.input(&self.categories) {
                Ok(input) => {
                    self.state = FormState::Submitting;
                    return Effects::NONE.send(
                        ComponentId::Feeds,
                        Command::Feeds(FeedsCommand::Subscribe(input)),
                    );
                }
                Err((field, message)) => self.show_error(field, message),
            },
            Field::Add => {
                self.focus = Field::Key(self.draft.labels.len());
                self.draft.labels.push(LabelRow::default());
            }
            Field::Label(i) => self.focus = Field::Key(i),
            Field::Value(i) | Field::Done(i) => {
                match self.draft.labels[i].entry(i, &self.categories) {
                    Ok(_) => {
                        self.state = FormState::Editing;
                        self.focus = Field::Label(i);
                    }
                    Err((field, message)) => self.show_error(field, message),
                }
            }
            Field::Remove(i) => {
                self.draft.labels.remove(i);
                self.state = FormState::Editing;
                self.focus = if i < self.draft.labels.len() {
                    Field::Label(i)
                } else {
                    Field::Add
                };
            }
            Field::Key(_) | Field::PollEvery => self.move_focus(Direction::Down),
        }
        Effects::NONE
    }

    fn show_error(&mut self, field: Field, message: String) {
        self.focus = field;
        self.state = FormState::Invalid(field, message);
    }
}

impl LabelRow {
    fn entry(
        &self,
        index: usize,
        categories: &Categories,
    ) -> Result<Option<(&str, Value)>, (Field, String)> {
        let key = self.key.line().trim();
        let raw = self.value.line();
        if key.is_empty() {
            return Err((
                Field::Key(index),
                "Enter a label key or remove this label.".into(),
            ));
        }
        let value = if key == "category" {
            if raw.trim().is_empty() {
                return Ok(None);
            }
            let category = Category::new(raw.to_owned()).map_err(|_| {
                (
                    Field::Value(index),
                    format!(
                        "Use at most {} bytes for a category name.",
                        Category::MAX_LEN
                    ),
                )
            })?;
            Value::String(categories.normalize(category).into_inner().into_owned())
        } else {
            match serde_json::from_str(raw) {
                Ok(value) => value,
                // An unfinished object, array or quoted string needs a correction.
                Err(err) if raw.trim_start().starts_with(['{', '[', '"']) => {
                    return Err((Field::Value(index), format!("Enter a JSON value: {err}")));
                }
                Err(_) => Value::String(raw.to_owned()),
            }
        };
        Ok(Some((key, value)))
    }
}

impl Draft {
    fn prompt_mut(&mut self, field: Field) -> Option<&mut Prompt> {
        match field {
            Field::Url => Some(&mut self.url),
            Field::Label(i) | Field::Value(i) => Some(&mut self.labels[i].value),
            Field::Key(i) => Some(&mut self.labels[i].key),
            Field::PollEvery => Some(&mut self.poll_every),
            _ => None,
        }
    }

    fn input(&self, categories: &Categories) -> Result<SubscribeFeedInput, (Field, String)> {
        let url = FeedUrl::parse(self.url.line().trim()).map_err(|_| {
            (
                Field::Url,
                "Enter a feed URL, e.g. https://example.org/feed.xml".into(),
            )
        })?;
        let crawl_policy = self.crawl_policy()?;
        let mut labels = SubscriptionLabels::default();
        for (i, row) in self.labels.iter().enumerate() {
            if let Some((key, value)) = row.entry(i, categories)?
                && labels.insert(key, value).is_some()
            {
                return Err((Field::Key(i), format!("Duplicate label key: {key}")));
            }
        }
        Ok(SubscribeFeedInput {
            url,
            labels,
            crawl_policy,
        })
    }

    fn crawl_policy(&self) -> Result<Option<CrawlPolicyInput>, (Field, String)> {
        let interval = self.poll_every.line().trim();
        if interval.is_empty() || interval.eq_ignore_ascii_case("default") {
            return Ok(None);
        }
        let seconds = polling_interval(interval).ok_or_else(|| {
            (
                Field::PollEvery,
                "Use a duration of at least 1s in whole seconds, e.g. 30m.".into(),
            )
        })?;
        Ok(Some(CrawlPolicyInput {
            polling: PollingPolicyInput::Interval { seconds },
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn infers_label_values_and_keeps_category_names_as_text() {
        let draft = Draft {
            url: "https://example.org/feed.xml".into(),
            labels: [
                ("priority", "3"),
                ("code", r#""3""#),
                ("issue", "003"),
                ("topic", "rust"),
                ("archived", "false"),
                ("tags", r#"["rust", "releases"]"#),
                ("category", "2026"),
            ]
            .into_iter()
            .map(|(key, value)| LabelRow {
                key: key.into(),
                value: value.into(),
            })
            .collect(),
            ..Draft::default()
        };
        let input = draft.input(&Categories::default_toml()).unwrap();
        assert_eq!(
            serde_json::to_value(input.labels).unwrap(),
            json!({
                "priority": 3, "code": "3", "issue": "003", "topic": "rust",
                "archived": false, "tags": ["rust", "releases"], "category": "2026"
            })
        );
    }
}
