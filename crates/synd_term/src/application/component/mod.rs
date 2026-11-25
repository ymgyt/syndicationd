use super::{Clock, overlay::Overlay};
use crate::keymap::Keymap;
use crate::{
    application::{ComponentId, Config},
    config::Categories,
    ui::{
        theme::Theme,
        widgets::{
            filter::FilterWidget,
            tabs::{Tab, TabsWidget},
        },
    },
};
use std::{rc::Rc, sync::Arc};
use unsubscribe::UnsubscribePopup;

mod commands;
mod unsubscribe;

mod feeds;
mod filter;
mod gh;
mod login;
mod reader;
mod view;
pub(super) use view::{FrameLayout, RenderContext};
mod shell;

pub(crate) use feeds::FeedsComponent;
pub(crate) use gh::GhComponent;
use login::LoginComponent;
use reader::ReaderComponent;
use shell::ShellComponent;

impl SyndUi {
    pub(in crate::application) fn get(
        &self,
        id: ComponentId,
    ) -> &dyn tuiframework::Component<Self> {
        match id {
            ComponentId::Shell => &self.shell,
            ComponentId::Tabs => &self.tabs,
            ComponentId::Login => &self.login,
            ComponentId::Filter => &self.filter,
            ComponentId::Entries => &self.feeds.entries,
            ComponentId::Reader => &self.reader,
            ComponentId::Feeds => &self.feeds,
            ComponentId::GhNotifications => &self.gh,
            ComponentId::UnsubscribePopup => &self.unsubscribe,
            ComponentId::GhFilterPopup => &self.gh.notifications.filter_popup,
        }
    }
    pub(in crate::application) fn get_mut(
        &mut self,
        id: ComponentId,
    ) -> &mut dyn tuiframework::Component<Self> {
        match id {
            ComponentId::Shell => &mut self.shell,
            ComponentId::Tabs => &mut self.tabs,
            ComponentId::Login => &mut self.login,
            ComponentId::Filter => &mut self.filter,
            ComponentId::Entries => &mut self.feeds.entries,
            ComponentId::Reader => &mut self.reader,
            ComponentId::Feeds => &mut self.feeds,
            ComponentId::GhNotifications => &mut self.gh,
            ComponentId::UnsubscribePopup => &mut self.unsubscribe,
            ComponentId::GhFilterPopup => &mut self.gh.notifications.filter_popup,
        }
    }
}

/// Owns the synd screens, their shared layout inputs, and the current overlays.
pub(crate) struct SyndUi {
    pub(super) categories: Rc<Categories>,
    pub(super) clock: Arc<dyn Clock>,
    pub(super) overlay: Overlay,
    pub(in crate::application) shell: ShellComponent,
    pub(in crate::application) login: LoginComponent,
    pub(in crate::application) filter: FilterWidget,
    pub(in crate::application) tabs: TabsWidget,
    pub(crate) feeds: FeedsComponent,
    reader: ReaderComponent,
    unsubscribe: UnsubscribePopup,
    pub(crate) gh: GhComponent,
}

impl SyndUi {
    pub(in crate::application) fn visible_tab(&self) -> Option<Tab> {
        self.login.has_api_access().then(|| self.tabs.current())
    }

    pub(super) fn new(
        config: Config,
        theme: Theme,
        categories: Categories,
        authentication_required: bool,
        clock: Arc<dyn Clock>,
    ) -> (Self, Keymap) {
        let categories = Rc::new(categories);
        let ui = Self {
            shell: ShellComponent::new(theme),
            login: LoginComponent::new(authentication_required),
            tabs: TabsWidget::new(&config.features),
            filter: FilterWidget::new(Rc::clone(&categories)),
            feeds: FeedsComponent::new(&config, Rc::clone(&categories)),
            reader: ReaderComponent::default(),
            gh: GhComponent::new(
                config.features.enable_gh_notification,
                Rc::clone(&categories),
            ),
            unsubscribe: UnsubscribePopup::default(),
            overlay: Overlay::default(),
            categories,
            clock,
        };
        (ui, config.keymaps.0)
    }
}
