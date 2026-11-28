use super::RenderContext;
use crate::application::component::Screens;
use crate::{
    application::{InFlightRequests, effects::Effects, request::synd_api_error_message},
    command::{Command, ShellCommand},
    event::{Event, EventKind, FeedRequestEvent, FeedsEvent, OperationError},
    keymap::Layer,
    operation::Operation,
    ui::{
        theme::{Palette, Theme},
        widgets::status::StatusLineWidget,
    },
};
use ratatui::widgets::{Block, Widget};
use tracing::{error, warn};
use tuiframework::Component;

/// Draws the background and status line and handles application-wide commands.
pub(in crate::application) struct ShellComponent {
    pub(in crate::application) theme: Theme,
    status: StatusLineWidget,
    in_flight: InFlightRequests,
}

impl ShellComponent {
    pub(super) fn new(theme: Theme) -> Self {
        Self {
            theme,
            status: StatusLineWidget::new(),
            in_flight: InFlightRequests::new(),
        }
    }

    fn rotate_theme(&mut self) {
        let palette = match self.theme.name {
            "ferra" => Palette::solarized_dark(),
            "solarized_dark" => Palette::helix(),
            "helix" => Palette::dracula(),
            "dracula" => Palette::eldritch(),
            _ => Palette::ferra(),
        };
        self.theme = Theme::new(palette).with_background(self.theme.background);
    }
}

impl ShellComponent {
    fn operation_error_message(error: &OperationError) -> String {
        match error {
            OperationError::OpenBrowser(error) => format!("open browser: {error}"),
            OperationError::OpenTextBrowser(error) => format!("open text browser: {error}"),
            OperationError::PersistCredential(error) => error.to_string(),
            OperationError::SetCredential(error) => synd_api_error_message(error),
        }
    }
}

impl Component<Screens> for ShellComponent {
    fn tick(&mut self) {
        self.in_flight.tick();
    }
    fn is_busy(&self) -> bool {
        !self.in_flight.is_empty()
    }
    fn on_input(&mut self, input: &crate::crossterm::event::Event, _: bool, _: &mut Effects) {
        use crate::crossterm::event::Event::{Key, Paste};
        if matches!(input, Key(_) | Paste(_)) {
            self.status.clear_error_message();
        }
    }
    fn apply_command(&mut self, command: Command, effects: &mut Effects) {
        let Command::Shell(action) = command else {
            return;
        };
        match action {
            ShellCommand::Quit => effects.quit(),
            ShellCommand::RotateTheme => self.rotate_theme(),
            ShellCommand::ForceRedraw => {
                effects.operation(Operation::ForceRedrawTerminal);
            }
        }
    }

    fn render(&self, frame: &mut ratatui::Frame<'_>, cx: &RenderContext<'_>) {
        let buf = frame.buffer_mut();
        Block::new()
            .style(cx.ui.theme.base)
            .render(cx.frame.area, buf);
        self.status.render(
            cx.frame.status,
            buf,
            &cx.ui,
            self.in_flight.status(),
            cx.keymap,
            cx.layers,
        );
    }

    fn key_layers(&self) -> &[Layer] {
        &[Layer::App, Layer::Global]
    }

    fn subscribes(&self, event: &Event) -> bool {
        [
            EventKind::RequestEmitted,
            EventKind::RequestCompleted,
            EventKind::Feeds,
            EventKind::CredentialRefreshFailed,
            EventKind::OperationFailed,
            EventKind::ErrorReported,
        ]
        .contains(&EventKind::from(event))
    }

    fn apply_event(&mut self, event: &Event, _: &mut Effects) {
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
            Event::ErrorReported(message) => self.status.set_error_message(message.clone()),
            Event::OperationFailed { error } => {
                let message = Self::operation_error_message(error);
                error!(%message, "application operation failed");
                self.status.set_error_message(message);
            }
            _ => {}
        }
    }
}
