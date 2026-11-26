use synd_feed::types::SubscriptionLabels;
use tuiframework::DriverContext;
use url::Url;

use crate::{
    event::{Event, OperationError},
    external_tools::ExternalTools,
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

    pub(super) fn open_feed_edition_editor(
        &self,
        prompt: &str,
        labels: SubscriptionLabels,
        cx: &mut DriverContext<Event>,
    ) {
        let event = match self.external_tools.open_editor(prompt) {
            Ok(input) => {
                cx.force_redraw();
                Event::FeedEditionEditorClosed { input, labels }
            }
            Err(error) => Event::OperationFailed {
                error: OperationError::OpenFeedEditionEditor(error),
            },
        };
        cx.emit(event);
    }
}
