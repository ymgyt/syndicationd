use crate::{
    application::{ComponentId, Config, effects::Effects},
    config::Categories,
    event::{Event, EventKind},
    keymap::{Layer, LayerStack},
    ui::{
        theme::Theme,
        widgets::{
            filter::FilterWidget,
            tabs::{Tab, TabsWidget},
        },
    },
};
use ratatui::buffer::Buffer;
use std::rc::Rc;
use unsubscribe::UnsubscribePopup;

mod commands;
mod unsubscribe;

mod feeds;
mod filter;
mod gh;
mod login;
mod view;
pub(super) use view::{FrameLayout, RenderContext};
mod shell;

pub(crate) use feeds::FeedsComponent;
pub(crate) use gh::GhComponent;
use login::LoginComponent;
use shell::ShellComponent;

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
            ComponentId::Tabs => &self.tabs,
            ComponentId::Login => &self.login,
            ComponentId::Filter => &self.filter,
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
            ComponentId::Tabs => &mut self.tabs,
            ComponentId::Login => &mut self.login,
            ComponentId::Filter => &mut self.filter,
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
    pub(in crate::application) shell: ShellComponent,
    pub(in crate::application) login: LoginComponent,
    pub(in crate::application) filter: FilterWidget,
    pub(in crate::application) tabs: TabsWidget,
    pub(crate) feeds: FeedsComponent,
    unsubscribe: UnsubscribePopup,
    pub(crate) gh: GhComponent,
}

impl Components {
    pub(in crate::application) fn visible_tab(&self) -> Option<Tab> {
        self.login.has_api_access().then(|| self.tabs.current())
    }

    pub(super) fn new(
        config: &Config,
        categories: &Rc<Categories>,
        theme: Theme,
        authentication_required: bool,
    ) -> Self {
        Self {
            shell: ShellComponent::new(theme),
            login: LoginComponent::new(authentication_required),
            tabs: TabsWidget::new(&config.features),
            filter: FilterWidget::new(Rc::clone(categories)),
            feeds: FeedsComponent::new(config, Rc::clone(categories)),
            gh: GhComponent::new(
                config.features.enable_gh_notification,
                Rc::clone(categories),
            ),
            unsubscribe: UnsubscribePopup::default(),
        }
    }
}
