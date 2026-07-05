use tracing::{debug, warn};
use url::Url;

use crate::{application::input_parser::InputParser, event::Event};

use super::DriverContext;

pub(super) struct InteractionDriver;

impl InteractionDriver {
    pub(super) fn open_feed_subscription_editor(cx: &mut DriverContext<'_>) -> Vec<Event> {
        match cx
            .handles
            .interactor
            .open_editor(InputParser::SUSBSCRIBE_FEED_PROMPT)
        {
            Ok(input) => {
                debug!("Got user modified feed subscription: {input}");
                cx.handles.terminal.force_redraw();
                vec![Event::FeedSubscriptionEditorClosed { input }]
            }
            Err(err) => {
                warn!("{err}");
                vec![Event::Error {
                    message: err.to_string(),
                }]
            }
        }
    }

    pub(super) fn open_feed_edition_editor(cx: &mut DriverContext<'_>, prompt: &str) -> Vec<Event> {
        match cx.handles.interactor.open_editor(prompt) {
            Ok(input) => {
                cx.handles.terminal.force_redraw();
                vec![Event::FeedEditionEditorClosed { input }]
            }
            Err(err) => {
                warn!("{err}");
                vec![Event::Error {
                    message: err.to_string(),
                }]
            }
        }
    }

    pub(super) fn open_browser(cx: &mut DriverContext<'_>, url: Url) -> Vec<Event> {
        match cx.handles.interactor.open_browser(url) {
            Ok(()) => Vec::new(),
            Err(err) => vec![Event::Error {
                message: format!("open browser: {err}"),
            }],
        }
    }

    pub(super) fn open_text_browser(cx: &mut DriverContext<'_>, url: Url) -> Vec<Event> {
        match cx.handles.interactor.open_text_browser(url) {
            Ok(()) => Vec::new(),
            Err(err) => vec![Event::Error {
                message: format!("open browser: {err}"),
            }],
        }
    }

    pub(super) fn force_redraw_terminal(cx: &mut DriverContext<'_>) -> Vec<Event> {
        cx.handles.terminal.force_redraw();
        Vec::new()
    }
}
