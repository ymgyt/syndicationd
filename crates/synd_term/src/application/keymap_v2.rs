use crossterm::event::KeyEvent;
use tracing::trace;

use crate::{
    application::Application,
    keymap::v2::{KeyStroke, KeymapResult, Layer, LayerStack},
    ui::widgets::{authentication::AuthenticateState, tabs::Tab},
};

impl Application {
    pub(super) fn handle_keymap_v2(&mut self, key: KeyEvent) {
        let layers = self.active_keymap_layers_v2();
        let result = self
            .key_resolver_v2
            .resolve(&self.keymaps_v2, &layers, KeyStroke::from(key));

        match result {
            KeymapResult::Matched(action) => {
                self.should_render();
                if let Some(command) = action.build_command() {
                    self.apply_command(command);
                }
            }
            KeymapResult::NoOp | KeymapResult::NotFound => {}
            KeymapResult::Pending { keys, candidates } => {
                trace!(?keys, ?candidates, "keymap sequence pending");
            }
            KeymapResult::Cancelled { keys } => {
                trace!(?keys, "keymap sequence cancelled");
            }
        }
    }

    fn active_keymap_layers_v2(&self) -> LayerStack {
        let mut layers = vec![Layer::App, Layer::Global];

        if self.components.shell.auth.state() != &AuthenticateState::Authenticated {
            layers.push(Layer::Login);
            return LayerStack::new(layers);
        }

        layers.push(Layer::Tabs);

        if self.components.github.is_filter_popup_open() {
            layers.push(Layer::GithubNotificationFilterPopup);
            return LayerStack::new(layers);
        }

        match self.components.shell.tabs.current() {
            Tab::Entries => layers.push(Layer::Entries),
            Tab::Feeds => layers.push(Layer::Feeds),
            Tab::GitHub => layers.push(Layer::GithubNotifications),
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

        LayerStack::new(layers)
    }
}
