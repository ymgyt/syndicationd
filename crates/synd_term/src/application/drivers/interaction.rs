use tracing::debug;
use tuiframework::DriverContext;
use url::Url;

use crate::{
    application::input_parser::InputParser,
    event::{Event, OperationError},
    external_tools::{ExternalTools, OpenEditorError},
};

/// Executes interactions that do not require direct terminal coordination.
pub(super) struct InteractionDriver {
    external_tools: Box<dyn ExternalTools>,
}

impl InteractionDriver {
    pub(super) fn new(external_tools: Box<dyn ExternalTools>) -> Self {
        Self { external_tools }
    }

    pub(super) fn open_browser(&self, url: Url) -> Option<Event> {
        self.external_tools
            .open_browser(url)
            .err()
            .map(|error| Event::OperationFailed {
                error: OperationError::OpenBrowser(error),
            })
    }

    pub(super) fn open_text_browser(&self, url: Url) -> Option<Event> {
        self.external_tools
            .open_text_browser(url)
            .err()
            .map(|error| Event::OperationFailed {
                error: OperationError::OpenTextBrowser(error),
            })
    }

    pub(super) fn open_feed_subscription_editor(&self, cx: &mut DriverContext<Event>) {
        self.open_editor(FeedSubscriptionEditor, cx);
    }

    pub(super) fn open_feed_edition_editor(&self, prompt: &str, cx: &mut DriverContext<Event>) {
        self.open_editor(FeedEditionEditor { prompt }, cx);
    }

    fn open_editor(&self, purpose: impl EditorPurpose, cx: &mut DriverContext<Event>) {
        let event = match self.external_tools.open_editor(purpose.prompt()) {
            Ok(input) => {
                cx.force_redraw();
                purpose.closed(input)
            }
            Err(error) => Event::OperationFailed {
                error: purpose.failed(error),
            },
        };
        cx.emit(event);
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
