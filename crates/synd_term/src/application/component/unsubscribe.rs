use super::RenderContext;
use super::Ui;
use crate::application::ComponentId;
use crate::{
    application::effects::Effects,
    command::{Command, UnsubscribeCommand, UnsubscribeTarget},
    keymap::{Layer, LayerStack},
    operation::Operation,
    ui::widgets::subscription::render_unsubscribe,
};
use tuiframework::Component;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum UnsubscribeSelection {
    #[default]
    Yes,
    No,
}

impl UnsubscribeSelection {
    fn toggle(self) -> Self {
        match self {
            UnsubscribeSelection::Yes => UnsubscribeSelection::No,
            UnsubscribeSelection::No => UnsubscribeSelection::Yes,
        }
    }
}

#[derive(Default)]
pub(super) struct UnsubscribePopup {
    selection: UnsubscribeSelection,
    target: Option<UnsubscribeTarget>,
}

impl UnsubscribePopup {
    pub(super) fn apply_command(&mut self, command: UnsubscribeCommand) -> Effects {
        match command {
            UnsubscribeCommand::Open(target) => {
                self.target = Some(target);
                Effects::NONE.send(ComponentId::UnsubscribePopup, Command::Open)
            }
            UnsubscribeCommand::ToggleSelection => {
                self.selection = self.selection.toggle();
                Effects::NONE
            }
            UnsubscribeCommand::Select => {
                let operation = self
                    .target
                    .take()
                    .filter(|_| self.selection == UnsubscribeSelection::Yes)
                    .map(|target| Operation::UnsubscribeFeed { url: target.url });
                Effects::from_operations(operation)
                    .send(ComponentId::UnsubscribePopup, Command::Close)
            }
            UnsubscribeCommand::Cancel => {
                self.target = None;
                Effects::NONE.send(ComponentId::UnsubscribePopup, Command::Close)
            }
        }
    }
}

impl Component<Ui> for UnsubscribePopup {
    fn apply_command(&mut self, command: Command) -> Effects {
        match command {
            Command::UnsubscribePopup(action) => self.apply_command(action),
            _ => Effects::NONE,
        }
    }

    fn render(&self, frame: &mut ratatui::Frame<'_>, cx: &RenderContext<'_>) {
        let buf = frame.buffer_mut();
        if let Some(target) = &self.target {
            render_unsubscribe(
                cx.frame.content,
                buf,
                &cx.ui,
                target,
                self.selection == UnsubscribeSelection::Yes,
            );
        }
    }

    fn key_layers(&self) -> &[Layer] {
        &[Layer::UnsubscribePopup]
    }

    fn mask_lower_layers(&self, layers: &mut LayerStack) {
        layers.cover(&[Layer::List, Layer::Feeds]);
    }
}
