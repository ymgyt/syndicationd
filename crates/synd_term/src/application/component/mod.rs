use crate::{
    application::{ComponentId, Config, effects::Effects},
    config::Categories,
    event::{Event, EventKind},
    keymap::{Layer, LayerStack},
    ui::{theme::Theme, widgets::filter::FilterWidget},
};
use ratatui::buffer::Buffer;
use std::rc::Rc;
use unsubscribe::UnsubscribePopup;

mod commands;
mod unsubscribe;

mod feeds;
mod filter;
mod gh;
mod view;
pub(super) use view::{FrameLayout, RenderContext};
mod shell;

pub(crate) use feeds::FeedsComponent;
pub(crate) use gh::GhComponent;
pub(crate) use shell::{AuthenticationState, ShellComponent};

pub(in crate::application) trait Component {
    fn render(&self, buf: &mut Buffer, cx: &RenderContext<'_>);
    fn key_layers(&self) -> &[Layer];
    fn mask_lower_layers(&self, _layers: &mut LayerStack) {}
    fn subscriptions(&self) -> &'static [EventKind] {
        &[]
    }
    fn apply_event(&mut self, _event: &Event) -> Effects {
        Effects::NONE
    }
}

impl Components {
    pub(in crate::application) fn get(&self, id: ComponentId) -> &dyn Component {
        match id {
            ComponentId::Shell => &self.shell,
            ComponentId::Tabs => &self.shell.tabs,
            ComponentId::Login => self.shell.authentication(),
            ComponentId::Filter => &self.shell.filter,
            ComponentId::Entries => &self.feeds.entries,
            ComponentId::Feeds => &self.feeds,
            ComponentId::GhNotifications => &self.gh,
            ComponentId::UnsubscribePopup => &self.unsubscribe,
            ComponentId::GhFilterPopup => &self.gh.notifications.filter_popup,
        }
    }
    pub(in crate::application) fn get_mut(&mut self, id: ComponentId) -> &mut dyn Component {
        match id {
            ComponentId::Shell => &mut self.shell,
            ComponentId::Tabs => &mut self.shell.tabs,
            ComponentId::Login => &mut self.shell.authentication,
            ComponentId::Filter => &mut self.shell.filter,
            ComponentId::Entries => &mut self.feeds.entries,
            ComponentId::Feeds => &mut self.feeds,
            ComponentId::GhNotifications => &mut self.gh,
            ComponentId::UnsubscribePopup => &mut self.unsubscribe,
            ComponentId::GhFilterPopup => &mut self.gh.notifications.filter_popup,
        }
    }
}

/// Top-level application state machine that coordinates child components.
pub(crate) struct Components {
    pub(crate) shell: ShellComponent,
    pub(crate) feeds: FeedsComponent,
    unsubscribe: UnsubscribePopup,
    pub(crate) gh: GhComponent,
}

impl Components {
    pub(super) fn new(
        config: &Config,
        categories: &Rc<Categories>,
        theme: Theme,
        authentication: AuthenticationState,
    ) -> Self {
        Self {
            shell: ShellComponent::new(
                &config.features,
                FilterWidget::new(Rc::clone(categories)),
                theme,
                authentication,
            ),
            feeds: FeedsComponent::new(config, Rc::clone(categories)),
            gh: GhComponent::new(
                config.features.enable_gh_notification,
                Rc::clone(categories),
            ),
            unsubscribe: UnsubscribePopup::default(),
        }
    }
}
