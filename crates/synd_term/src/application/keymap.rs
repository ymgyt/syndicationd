use crossterm::event::KeyEvent;
use tracing::trace;

use crate::{
    command::Command,
    keymap::{KeymapResult, Layer, LayerStack},
    ui::widgets::tabs::Tab,
};

use super::{Application, component::AuthenticationState};

impl Application {
    pub(super) fn resolve_keymap(&mut self, key: KeyEvent) -> Option<Command> {
        self.components.filter_keymap().install(&mut self.keymap);
        match self.keymap.resolve(&self.active_keymap_layers(), key) {
            KeymapResult::Matched(action) => Some(action.build_command()),
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

    pub(super) fn active_keymap_layers(&self) -> LayerStack {
        let mut layers = LayerStack::empty();
        layers.push(Layer::App);
        layers.push(Layer::Global);

        if !matches!(
            self.components.shell.authentication(),
            AuthenticationState::NotRequired | AuthenticationState::Authenticated
        ) {
            layers.push(Layer::Login);
            return layers;
        }

        layers.push(Layer::Tabs);

        if self.components.gh.is_filter_popup_open() {
            layers.push(Layer::GhNotificationFilterPopup);
            return layers;
        }

        match self.components.shell.tabs.current() {
            Tab::Entries => layers.push(Layer::Entries),
            Tab::Feeds => layers.push(Layer::Feeds),
            Tab::Gh => layers.push(Layer::GhNotifications),
        }

        layers.push(Layer::Filter);

        if self.components.shell.filter.is_category_filtering_active() {
            layers.push(Layer::CategoryFilter);
        }
        if self.components.shell.filter.is_search_active() {
            layers.push(Layer::SearchPrompt);
        }
        if self.components.is_feed_unsubscription_popup_open() {
            layers.push(Layer::UnsubscribePopup);
        }

        layers
    }
}
