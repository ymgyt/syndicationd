use super::RenderContext;
use super::Screens;
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
    pub(super) fn apply_command(&mut self, command: UnsubscribeCommand, effects: &mut Effects) {
        match command {
            UnsubscribeCommand::Open(target) => {
                self.target = Some(target);
                effects.open(crate::application::overlay::OverlayId::Feeds);
            }
            UnsubscribeCommand::ToggleSelection => {
                self.selection = self.selection.toggle();
            }
            UnsubscribeCommand::Select => {
                let operation = self
                    .target
                    .take()
                    .filter(|_| self.selection == UnsubscribeSelection::Yes)
                    .map(|target| Operation::UnsubscribeFeed { url: target.url });
                effects.operations(operation);
                effects.close();
            }
            UnsubscribeCommand::Cancel => {
                self.target = None;
                effects.close();
            }
        }
    }
}

impl Component<Screens> for UnsubscribePopup {
    fn apply_command(&mut self, command: Command, effects: &mut Effects) {
        if let Command::UnsubscribePopup(action) = command {
            self.apply_command(action, effects);
        }
    }

    fn subscribes(&self, event: &crate::event::Event) -> bool {
        matches!(event, crate::event::Event::UnsubscriptionRequested(_))
    }
    fn apply_event(&mut self, event: &crate::event::Event, effects: &mut Effects) {
        if let crate::event::Event::UnsubscriptionRequested(target) = event {
            self.apply_command(UnsubscribeCommand::Open(target.clone()), effects);
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
