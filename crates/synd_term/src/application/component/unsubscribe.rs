use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    text::{Line, Span},
    widgets::{Paragraph, Widget, Wrap},
};
use tuiframework::Component;

use super::{RenderContext, Screens};
use crate::{
    application::{
        RequestError, RequestKind, effects::Effects, overlay::OverlayId,
        request::synd_api_error_message,
    },
    command::{Command, UnsubscribeCommand, UnsubscribeTarget},
    event::Event,
    keymap::{Layer, LayerStack},
    operation::Operation,
    ui::widgets::Button,
};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum ConfirmationState {
    #[default]
    Cancel,
    Unsubscribe,
    Submitting,
}

/// Holds the chosen feed until the user cancels or the server confirms removal.
#[derive(Default)]
pub(super) struct UnsubscribePopup {
    active: Option<Confirmation>,
}

struct Confirmation {
    target: UnsubscribeTarget,
    state: ConfirmationState,
    error: Option<String>,
}

impl Component<Screens> for UnsubscribePopup {
    fn apply_command(&mut self, command: Command, effects: &mut Effects) {
        let Command::UnsubscribePopup(action) = command else {
            return;
        };
        let Some(confirmation) = &mut self.active else {
            return;
        };
        if confirmation.state == ConfirmationState::Submitting {
            return;
        }
        match action {
            UnsubscribeCommand::ToggleSelection => {
                confirmation.state = if confirmation.state == ConfirmationState::Cancel {
                    ConfirmationState::Unsubscribe
                } else {
                    ConfirmationState::Cancel
                };
            }
            UnsubscribeCommand::Select if confirmation.state == ConfirmationState::Unsubscribe => {
                confirmation.state = ConfirmationState::Submitting;
                confirmation.error = None;
                effects.operation(Operation::UnsubscribeFeed {
                    url: confirmation.target.url.clone(),
                });
            }
            UnsubscribeCommand::Cancel | UnsubscribeCommand::Select => {
                self.active = None;
                effects.close();
            }
        }
    }

    fn subscribes(&self, event: &Event) -> bool {
        match event {
            Event::UnsubscriptionRequested(_) => true,
            Event::RequestCompleted {
                kind: RequestKind::UnsubscribeFeed { url },
                ..
            } => self.active.as_ref().is_some_and(|confirmation| {
                confirmation.state == ConfirmationState::Submitting
                    && confirmation.target.url == *url
            }),
            _ => false,
        }
    }

    fn apply_event(&mut self, event: &Event, effects: &mut Effects) {
        match (event, &mut self.active) {
            (Event::UnsubscriptionRequested(target), _) => {
                self.active = Some(Confirmation {
                    target: target.clone(),
                    state: ConfirmationState::Cancel,
                    error: None,
                });
                effects.open(OverlayId::Feeds);
            }
            (Event::RequestCompleted { result: Ok(()), .. }, _) => {
                self.active = None;
                effects.close();
            }
            (
                Event::RequestCompleted {
                    result: Err(RequestError::SyndApi(error)),
                    ..
                },
                Some(confirmation),
            ) => {
                confirmation.state = ConfirmationState::Unsubscribe;
                confirmation.error = Some(format!(
                    "Could not unsubscribe: {}",
                    synd_api_error_message(error)
                ));
            }
            _ => {}
        }
    }

    fn render(&self, frame: &mut Frame<'_>, cx: &RenderContext<'_>) {
        if let Some(confirmation) = &self.active {
            confirmation.render(frame, cx);
        }
    }

    fn key_layers(&self) -> &[Layer] {
        if self
            .active
            .as_ref()
            .is_some_and(|confirmation| confirmation.state == ConfirmationState::Submitting)
        {
            &[]
        } else {
            &[Layer::UnsubscribePopup]
        }
    }

    fn mask_lower_layers(&self, layers: &mut LayerStack) {
        layers.truncate_after(Layer::App);
    }
}

impl Confirmation {
    fn render(&self, frame: &mut Frame<'_>, cx: &RenderContext<'_>) {
        let target = &self.target;
        let dialog = cx.frame.dialog();
        let mut lines = Vec::new();
        if let Some(title) = &target.title {
            lines.push(Line::styled(title.as_str(), cx.ui.theme.application_title));
        }
        lines.push(Line::raw(target.url.as_str()));
        if let Some(error) = &self.error {
            lines.push(Line::default());
            lines.push(Line::styled(error.as_str(), cx.ui.theme.error.message));
        }
        let info = Paragraph::new(lines).wrap(Wrap { trim: false });
        let height = info
            .line_count(dialog.inner_width())
            .try_into()
            .unwrap_or(u16::MAX);
        let buf = frame.buffer_mut();
        let (body, footer) = dialog.render(buf, cx.ui.theme, "Unsubscribe", height, 0);
        let scroll = if self.error.is_some() {
            height.saturating_sub(body.height)
        } else {
            0
        };
        info.scroll((scroll, 0)).render(body, buf);
        let [cancel, unsubscribe] =
            Layout::horizontal([Constraint::Length(10), Constraint::Fill(1)]).areas(footer);
        let theme = cx.ui.theme;
        Line::from(Button::new(
            "Cancel",
            self.state == ConfirmationState::Cancel,
            theme,
        ))
        .render(cancel, buf);
        let action = if self.state == ConfirmationState::Submitting {
            Line::from(Span::styled("Unsubscribing…", theme.tabs_selected))
        } else {
            Button::new(
                "Unsubscribe",
                self.state == ConfirmationState::Unsubscribe,
                theme,
            )
            .into()
        };
        action.right_aligned().render(unsubscribe, buf);
    }
}
