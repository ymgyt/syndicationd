use tracing::{debug, warn};
use url::Url;

use crate::{
    application::input_parser::InputParser, event::Event, interact::Interact, terminal::Terminal,
};

/// Executes interactions that hand the screen to another process:
/// external editor and browsers.
pub(super) struct InteractionDriver {
    pub(super) interactor: Box<dyn Interact>,
}

impl InteractionDriver {
    pub(super) fn open_feed_subscription_editor(&self, terminal: &mut Terminal) -> Event {
        match self
            .interactor
            .open_editor(InputParser::SUSBSCRIBE_FEED_PROMPT)
        {
            Ok(input) => {
                debug!("Got user modified feed subscription: {input}");
                terminal.force_redraw();
                Event::FeedSubscriptionEditorClosed { input }
            }
            Err(err) => {
                warn!("{err}");
                Event::Error {
                    message: err.to_string(),
                }
            }
        }
    }

    pub(super) fn open_feed_edition_editor(&self, terminal: &mut Terminal, prompt: &str) -> Event {
        match self.interactor.open_editor(prompt) {
            Ok(input) => {
                terminal.force_redraw();
                Event::FeedEditionEditorClosed { input }
            }
            Err(err) => {
                warn!("{err}");
                Event::Error {
                    message: err.to_string(),
                }
            }
        }
    }

    pub(super) fn open_browser(&self, url: Url) -> Option<Event> {
        self.interactor
            .open_browser(url)
            .err()
            .map(|err| Event::Error {
                message: format!("open browser: {err}"),
            })
    }

    pub(super) fn open_text_browser(&self, url: Url) -> Option<Event> {
        self.interactor
            .open_text_browser(url)
            .err()
            .map(|err| Event::Error {
                message: format!("open browser: {err}"),
            })
    }
}
