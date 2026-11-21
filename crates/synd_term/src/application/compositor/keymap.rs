use crossterm::event::KeyEvent;
use tracing::trace;

use crate::{
    application::ComponentId,
    command::Command,
    keymap::{KeymapResult, Layer, LayerStack},
};

use super::Compositor;

impl Compositor {
    fn key_layers(&self) -> LayerStack {
        let mut layers = LayerStack::default();
        for id in self.overlay.visible(self.components.shell.visible_tab()) {
            let component = self.components.get(id);
            component.mask_lower_layers(&mut layers);
            layers.extend(component.key_layers());
        }
        layers
    }

    pub(super) fn sync_keymap(&mut self) {
        self.keymap.sync_layers(&self.key_layers());
    }

    fn owner(&self, layer: Layer) -> ComponentId {
        self.overlay
            .visible(self.components.shell.visible_tab())
            .rev()
            .find(|id| self.components.get(*id).key_layers().contains(&layer))
            .expect("matched layer has a visible owner")
    }

    pub(super) fn resolve_keymap(&mut self, key: KeyEvent) -> Option<Command> {
        self.components
            .shell
            .install_filter_keymap(&mut self.keymap);
        match self.keymap.resolve(&self.key_layers(), key) {
            KeymapResult::Matched { layer, action } => {
                Some(action.build_command(self.owner(layer)))
            }
            KeymapResult::NotFound => None,
            KeymapResult::Pending { keys, candidates } => {
                trace!(?keys, ?candidates, "keymap sequence pending");
                None
            }
            KeymapResult::Cancelled { keys } => {
                trace!(?keys, "keymap sequence cancelled");
                None
            }
        }
    }
}
