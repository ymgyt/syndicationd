use tracing::debug;
use url::Url;

use crate::{
    application::input_parser::InputParser,
    event::{Event, OperationError},
    interact::{Interact, OpenEditorError},
    terminal::Terminal,
};

/// Executes interactions that do not require direct terminal coordination.
pub(super) struct InteractionDriver {
    interactor: Box<dyn Interact>,
}

impl InteractionDriver {
    pub(super) fn new(interactor: Box<dyn Interact>) -> Self {
        Self { interactor }
    }

    pub(super) fn open_browser(&self, url: Url) -> Option<Event> {
        self.interactor
            .open_browser(url)
            .err()
            .map(|error| Event::OperationFailed {
                error: OperationError::OpenBrowser(error),
            })
    }

    pub(super) fn open_text_browser(&self, url: Url) -> Option<Event> {
        self.interactor
            .open_text_browser(url)
            .err()
            .map(|error| Event::OperationFailed {
                error: OperationError::OpenTextBrowser(error),
            })
    }
}

/// Coordinates editor execution with the redraw performed after a successful edit.
pub(super) struct TerminalInteraction<'a> {
    interaction: &'a InteractionDriver,
    terminal: &'a mut Terminal,
}

impl<'a> TerminalInteraction<'a> {
    pub(super) fn new(interaction: &'a InteractionDriver, terminal: &'a mut Terminal) -> Self {
        Self {
            interaction,
            terminal,
        }
    }

    pub(super) fn open_feed_subscription_editor(self) -> Event {
        self.open_editor(FeedSubscriptionEditor)
    }

    pub(super) fn open_feed_edition_editor(self, prompt: &str) -> Event {
        self.open_editor(FeedEditionEditor { prompt })
    }

    fn open_editor<P>(self, purpose: P) -> Event
    where
        P: EditorPurpose,
    {
        match self.interaction.interactor.open_editor(purpose.prompt()) {
            Ok(input) => {
                self.terminal.force_redraw();
                purpose.closed(input)
            }
            Err(error) => Event::OperationFailed {
                error: purpose.failed(error),
            },
        }
    }
}

/// Defines the prompt and protocol result for one editor-backed operation.
trait EditorPurpose {
    fn prompt(&self) -> &str;
    fn closed(self, input: String) -> Event;
    fn failed(self, error: OpenEditorError) -> OperationError;
}

struct FeedSubscriptionEditor;

impl EditorPurpose for FeedSubscriptionEditor {
    fn prompt(&self) -> &str {
        InputParser::SUSBSCRIBE_FEED_PROMPT
    }

    fn closed(self, input: String) -> Event {
        debug!("Got user modified feed subscription: {input}");
        Event::FeedSubscriptionEditorClosed { input }
    }

    fn failed(self, error: OpenEditorError) -> OperationError {
        OperationError::OpenFeedSubscriptionEditor(error)
    }
}

struct FeedEditionEditor<'a> {
    prompt: &'a str,
}

impl EditorPurpose for FeedEditionEditor<'_> {
    fn prompt(&self) -> &str {
        self.prompt
    }

    fn closed(self, input: String) -> Event {
        Event::FeedEditionEditorClosed { input }
    }

    fn failed(self, error: OpenEditorError) -> OperationError {
        OperationError::OpenFeedEditionEditor(error)
    }
}
