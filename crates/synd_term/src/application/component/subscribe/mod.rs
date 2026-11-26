use std::rc::Rc;

use serde_json::Value;
use synd_client::payload::{CrawlPolicyInput, PollingPolicyInput, SubscribeFeedInput};
use synd_feed::types::{Category, FeedUrl, SubscriptionLabels};
use tuiframework::Component;

use super::{ComponentId, RenderContext, Ui};
use crate::{
    application::{Direction, effects::Effects, input_parser::polling_interval},
    command::{Command, FeedsCommand, ShellCommand, SubscribeCommand},
    config::Categories,
    keymap::Layer,
    ui::widgets::prompt::Prompt,
};

mod render;
use render::Row;

/// Edits a subscription until Feeds accepts the draft.
pub(super) struct SubscribeForm {
    categories: Rc<Categories>,
    draft: Draft,
    focus: Field,
}

#[derive(Default)]
struct Draft {
    url: Prompt,
    category: Prompt,
    labels: Vec<LabelRow>,
    poll_every: Prompt,
}

#[derive(Default)]
struct LabelRow {
    key: Prompt,
    value: Prompt,
    json: bool,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Field {
    Url,
    Category,
    Key(usize),
    Value(usize),
    Format(usize),
    Remove(usize),
    Add,
    PollEvery,
    Submit,
}

impl Component<Ui> for SubscribeForm {
    fn key_layers(&self) -> &[Layer] {
        &[Layer::TextInput, Layer::Subscribe]
    }

    fn apply_command(&mut self, command: Command) -> Effects {
        match command {
            Command::Subscribe(SubscribeCommand::Open) => {
                self.draft = Draft::default();
                self.focus = Field::Url;
                return Effects::NONE.send(ComponentId::Subscribe, Command::Open);
            }
            Command::Subscribe(SubscribeCommand::Cancel) => {
                return Effects::NONE.send(ComponentId::Subscribe, Command::Close);
            }
            Command::Subscribe(SubscribeCommand::Confirm) => return self.confirm(),
            Command::Subscribe(SubscribeCommand::Move(direction)) => self.move_focus(direction),
            Command::Subscribe(SubscribeCommand::AlreadySubscribed) => {
                return self.show_error(Field::Url, "Already subscribed to this URL".into());
            }
            Command::Prompt(action) => {
                if let Some(prompt) = self.draft.prompt_mut(self.focus) {
                    prompt.apply(&action);
                }
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
        }
    }

    fn move_focus(&mut self, direction: Direction) {
        let rows = self.draft.rows();
        let fields: Vec<_> = rows.iter().flat_map(Row::fields).copied().collect();
        let current = fields
            .iter()
            .position(|&field| field == self.focus)
            .expect("focused field exists");
        self.focus = fields[direction.apply(current, fields.len())];
    }

    fn confirm(&mut self) -> Effects {
        match self.focus {
            Field::Url | Field::Submit => match self.draft.input(&self.categories) {
                Ok(input) => {
                    return Effects::NONE.send(
                        ComponentId::Feeds,
                        Command::Feeds(FeedsCommand::Subscribe(input)),
                    );
                }
                Err((field, message)) => return self.show_error(field, message),
            },
            Field::Add => {
                self.focus = Field::Key(self.draft.labels.len());
                self.draft.labels.push(LabelRow::default());
            }
            Field::Remove(index) => {
                self.draft.labels.remove(index);
                self.focus = if index < self.draft.labels.len() {
                    Field::Key(index)
                } else {
                    Field::Add
                };
            }
            Field::Format(index) => {
                let row = &mut self.draft.labels[index];
                row.json = !row.json;
            }
            _ => self.move_focus(Direction::Down),
        }
        Effects::NONE
    }

    fn show_error(&mut self, field: Field, message: String) -> Effects {
        self.focus = field;
        Effects::NONE.send(
            ComponentId::Shell,
            Command::Shell(ShellCommand::ShowError(message)),
        )
    }
}

impl Draft {
    fn prompt_mut(&mut self, field: Field) -> Option<&mut Prompt> {
        match field {
            Field::Url => Some(&mut self.url),
            Field::Category => Some(&mut self.category),
            Field::Key(i) => Some(&mut self.labels[i].key),
            Field::Value(i) => Some(&mut self.labels[i].value),
            Field::PollEvery => Some(&mut self.poll_every),
            Field::Format(_) | Field::Remove(_) | Field::Add | Field::Submit => None,
        }
    }

    fn input(&self, categories: &Categories) -> Result<SubscribeFeedInput, (Field, String)> {
        let url = FeedUrl::parse(self.url.line().trim()).map_err(|_| {
            (
                Field::Url,
                "Enter a feed URL, e.g. https://example.org/feed.xml".into(),
            )
        })?;
        Ok(SubscribeFeedInput {
            url,
            labels: self.subscription_labels(categories)?,
            crawl_policy: self.crawl_policy()?,
        })
    }

    fn subscription_labels(
        &self,
        categories: &Categories,
    ) -> Result<SubscriptionLabels, (Field, String)> {
        let mut labels = SubscriptionLabels::default();
        if !self.category.line().trim().is_empty() {
            let category = Category::new(self.category.line().to_owned())
                .map_err(|err| (Field::Category, format!("Category: {err}")))?;
            labels.set_category(categories.normalize(category));
        }
        for (i, row) in self.labels.iter().enumerate() {
            let key = row.key.line().trim();
            if key.is_empty() {
                return Err((Field::Key(i), "Enter a label key or remove the row".into()));
            }
            let value = if row.json {
                serde_json::from_str(row.value.line())
                    .map_err(|err| (Field::Value(i), format!("Invalid JSON: {err}")))?
            } else {
                Value::String(row.value.line().to_owned())
            };
            if labels.insert(key, value).is_some() {
                return Err((Field::Key(i), format!("Duplicate label key: {key}")));
            }
        }
        Ok(labels)
    }

    fn crawl_policy(&self) -> Result<Option<CrawlPolicyInput>, (Field, String)> {
        let interval = self.poll_every.line().trim();
        if interval.is_empty() || interval.eq_ignore_ascii_case("default") {
            return Ok(None);
        }
        let seconds = polling_interval(interval).ok_or_else(|| {
            (
                Field::PollEvery,
                "Enter default or a positive whole-second interval, e.g. 30m or 2h".into(),
            )
        })?;
        Ok(Some(CrawlPolicyInput {
            polling: PollingPolicyInput::Interval { seconds },
        }))
    }
}
