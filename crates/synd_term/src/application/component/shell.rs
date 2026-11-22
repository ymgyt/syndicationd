use super::{Component, RenderContext};
use crate::{
    application::{InFlightRequests, effects::Effects, request::synd_api_error_message},
    command::ShellCommand,
    event::{Event, EventKind, FeedRequestEvent, FeedsEvent, OperationError},
    keymap::Layer,
    operation::Operation,
    ui::{
        theme::{Palette, Theme},
        widgets::status::StatusLineWidget,
    },
};
use ratatui::{
    buffer::Buffer,
    widgets::{Block, Widget},
};
use tracing::{error, warn};

/// Draws the background and status line and handles application-wide commands.
pub(in crate::application) struct ShellComponent {
    pub(in crate::application) theme: Theme,
    pub(in crate::application) should_quit: bool,
    status: StatusLineWidget,
    in_flight: InFlightRequests,
}

impl ShellComponent {
    pub(super) fn new(theme: Theme) -> Self {
        Self {
            theme,
            should_quit: false,
            status: StatusLineWidget::new(),
            in_flight: InFlightRequests::new(),
        }
    }

    pub(in crate::application) fn tick(&mut self) {
        self.in_flight.tick();
    }

    pub(in crate::application) fn is_busy(&self) -> bool {
        !self.in_flight.is_empty()
    }

    pub(in crate::application) fn dismiss_error(&mut self) {
        self.status.clear_error_message();
    }

    pub(super) fn apply_command(&mut self, command: ShellCommand) -> Effects {
        match command {
            ShellCommand::ShowError(message) => self.status.set_error_message(message),
            ShellCommand::Quit => self.should_quit = true,
            ShellCommand::RotateTheme => self.rotate_theme(),
            ShellCommand::ForceRedraw => {
                return Effects::operations(Operation::ForceRedrawTerminal);
            }
        }
        Effects::NONE
    }

    fn rotate_theme(&mut self) {
        let palette = match self.theme.name {
            "ferra" => Palette::solarized_dark(),
            "solarized_dark" => Palette::helix(),
            "helix" => Palette::dracula(),
            "dracula" => Palette::eldritch(),
            _ => Palette::ferra(),
        };
        self.theme = Theme::with_palette(palette);
    }
}

impl ShellComponent {
    fn operation_error_message(error: &OperationError) -> String {
        match error {
            OperationError::OpenFeedSubscriptionEditor(error)
            | OperationError::OpenFeedEditionEditor(error) => error.to_string(),
            OperationError::OpenBrowser(error) => format!("open browser: {error}"),
            OperationError::OpenTextBrowser(error) => format!("open text browser: {error}"),
            OperationError::PersistCredential(error) => error.to_string(),
            OperationError::SetCredential(error) => synd_api_error_message(error),
        }
    }
}

impl Component for ShellComponent {
    fn render(&self, buf: &mut Buffer, cx: &RenderContext<'_>) {
        Block::new()
            .style(cx.ui.theme.base)
            .render(cx.frame.area, buf);
        self.status.render(
            cx.frame.status,
            buf,
            &cx.ui,
            self.in_flight.status(),
            cx.keymap,
            &cx.layers,
        );
    }

    fn key_layers(&self) -> &[Layer] {
        &[Layer::App, Layer::Global]
    }

    fn subscriptions(&self) -> &'static [EventKind] {
        &[
            EventKind::RequestEmitted,
            EventKind::RequestCompleted,
            EventKind::Feeds,
            EventKind::CredentialRefreshFailed,
            EventKind::OperationFailed,
        ]
    }

    fn apply_event(&mut self, event: &Event) -> Effects {
        match event {
            Event::RequestEmitted { request_id, kind } => {
                self.in_flight.register(*request_id, kind.clone());
            }
            Event::Feeds(FeedsEvent::Request {
                request_id,
                event: FeedRequestEvent::TimelineWindowChunkFetched { entries, .. },
            }) => {
                self.in_flight
                    .advance_timeline_window(*request_id, entries.len());
            }
            Event::RequestCompleted {
                request_id, kind, ..
            } => {
                assert_eq!(
                    &self.in_flight.complete(*request_id),
                    kind,
                    "driver completed a different request kind"
                );
            }
            Event::CredentialRefreshFailed { error } => {
                warn!(%error, "credential refresh failed");
                self.status.set_error_message(error.to_string());
            }
            Event::OperationFailed { error } => {
                let message = Self::operation_error_message(error);
                error!(%message, "application operation failed");
                self.status.set_error_message(message);
            }
            _ => {}
        }
        Effects::NONE
    }
}
