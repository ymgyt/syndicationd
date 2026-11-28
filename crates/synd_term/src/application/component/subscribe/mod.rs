use std::rc::Rc;

use synd_feed::types::FeedUrl;
use tuiframework::Component;

use super::{RenderContext, Screens};
use crate::{
    application::{
        Direction, RequestError, RequestKind, effects::Effects, request::synd_api_error_message,
    },
    command::{Command, SubscribeCommand, SubscriptionMode},
    config::Categories,
    event::Event,
    keymap::{Layer, LayerStack},
};

mod draft;
mod render;

use draft::{Draft, LabelRow};

/// Owns the draft until the subscription is accepted or the user cancels.
pub(super) struct SubscriptionForm {
    categories: Rc<Categories>,
    draft: Draft,
    focus: Field,
    mode: SubscriptionMode,
    state: FormState,
}

enum FormState {
    Editing,
    Invalid(Field, String),
    Submitting(FeedUrl),
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

impl Component<Screens> for SubscriptionForm {
    fn key_layers(&self) -> &[Layer] {
        match self.state {
            FormState::Submitting(_) => &[],
            _ => &[Layer::TextInput, Layer::Subscribe],
        }
    }

    fn mask_lower_layers(&self, layers: &mut LayerStack) {
        layers.truncate_after(Layer::App);
    }

    fn apply_command(&mut self, command: Command, effects: &mut Effects) {
        match command {
            _ if matches!(self.state, FormState::Submitting(_)) => {}
            Command::Subscribe(SubscribeCommand::Open) => {
                self.open(
                    SubscriptionMode::Create,
                    Draft::default(),
                    Field::Url,
                    effects,
                );
            }
            Command::Subscribe(SubscribeCommand::Cancel) => {
                effects.close();
            }
            Command::Subscribe(SubscribeCommand::Confirm) => self.confirm(effects),
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
    }

    fn on_input(
        &mut self,
        input: &crate::crossterm::event::Event,
        frontmost: bool,
        effects: &mut Effects,
    ) {
        if frontmost && let crate::crossterm::event::Event::Paste(text) = input {
            self.apply_command(
                Command::Prompt(crate::command::PromptAction::Paste(text.clone())),
                effects,
            );
        }
    }

    fn subscribes(&self, event: &Event) -> bool {
        match event {
            Event::SubscriptionEditRequested { .. } => true,
            Event::SubscriptionDuplicate => {
                matches!(self.state, FormState::Submitting(_))
                    && self.mode == SubscriptionMode::Create
            }
            Event::RequestCompleted {
                kind: RequestKind::SubscribeFeed { url },
                ..
            } => {
                matches!(&self.state, FormState::Submitting(pending) if pending == url)
            }
            _ => false,
        }
    }

    fn apply_event(&mut self, event: &Event, effects: &mut Effects) {
        match event {
            Event::SubscriptionEditRequested {
                url,
                polling,
                labels,
            } => {
                let draft = Draft::edit(url, polling, labels);
                let focus = draft
                    .labels
                    .iter()
                    .position(|row| row.key.line() == "category")
                    .map_or(Field::PollEvery, Field::Label);
                self.open(SubscriptionMode::Edit, draft, focus, effects);
            }
            Event::SubscriptionDuplicate => {
                self.show_error(Field::Url, "Already subscribed to this URL".into());
            }
            Event::RequestCompleted { result: Ok(()), .. } => {
                self.state = FormState::Editing;
                effects.close();
            }
            Event::RequestCompleted {
                result: Err(RequestError::SyndApi(error)),
                ..
            } => {
                tracing::error!(%error, "subscription request failed");
                self.show_error(
                    Field::Submit,
                    format!(
                        "Could not {}: {}",
                        if self.mode == SubscriptionMode::Edit {
                            "save"
                        } else {
                            "subscribe"
                        },
                        synd_api_error_message(error)
                    ),
                );
            }
            _ => {}
        }
    }

    fn render(&self, frame: &mut ratatui::Frame<'_>, cx: &RenderContext<'_>) {
        self.render_form(frame, cx);
    }
}

impl SubscriptionForm {
    pub(super) fn new(categories: Rc<Categories>) -> Self {
        Self {
            categories,
            draft: Draft::default(),
            mode: SubscriptionMode::Create,
            focus: Field::Url,
            state: FormState::Editing,
        }
    }

    fn open(&mut self, mode: SubscriptionMode, draft: Draft, focus: Field, effects: &mut Effects) {
        self.mode = mode;
        self.draft = draft;
        self.focus = focus;
        self.state = FormState::Editing;
        effects.open(crate::application::overlay::OverlayId::Feeds);
    }

    fn move_focus(&mut self, direction: Direction) {
        let fields = self.focus_order();
        let current = fields
            .iter()
            .position(|&field| field == self.focus)
            .expect("focused control");
        self.focus = fields[direction.apply(current, fields.len())];
    }

    fn confirm(&mut self, effects: &mut Effects) {
        match self.focus {
            Field::Url | Field::Submit => match self.draft.input(&self.categories) {
                Ok(input) => {
                    self.state = FormState::Submitting(input.url.clone());
                    effects.emit(Event::SubscriptionSubmitted {
                        mode: self.mode,
                        input,
                    });
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
    }

    fn show_error(&mut self, field: Field, message: String) {
        self.focus = field;
        self.state = FormState::Invalid(field, message);
    }
}
