use std::{rc::Rc, sync::Arc};

use strum::IntoEnumIterator as _;
use tuiframework::Component;

use super::{Clock, ComponentId, Config};
use crate::{
    command::Command,
    config::Categories,
    event::Event,
    keymap::{Keymap, Layer},
    operation::Operation,
    ui::{
        theme::Theme,
        widgets::{
            filter::FilterWidget,
            tabs::{Tab, TabsWidget},
        },
    },
};

mod commands;
mod subscribe;
mod unsubscribe;

mod feeds;
mod filter;
mod gh;
mod login;
mod reader;
mod render;
mod shell;
pub(super) use render::RenderContext;

pub(crate) use feeds::FeedsComponent;
pub(crate) use gh::GhComponent;
use login::LoginComponent;
use reader::ReaderComponent;
use shell::ShellComponent;
use unsubscribe::UnsubscribePopup;

/// Owns the synd screens, their shared layout inputs, and domain state.
pub(crate) struct Screens {
    categories: Rc<Categories>,
    clock: Arc<dyn Clock>,
    pub(in crate::application) shell: ShellComponent,
    pub(in crate::application) login: LoginComponent,
    pub(in crate::application) filter: FilterWidget,
    pub(in crate::application) tabs: TabsWidget,
    pub(crate) feeds: FeedsComponent,
    reader: ReaderComponent,
    subscribe: subscribe::SubscribeForm,
    unsubscribe: UnsubscribePopup,
    pub(crate) gh: GhComponent,
}

impl Screens {
    pub(in crate::application) fn visible_tab(&self) -> Option<Tab> {
        self.login.has_api_access().then(|| self.tabs.current())
    }

    pub(super) fn new(
        config: &Config,
        theme: Theme,
        categories: Categories,
        authentication_required: bool,
        clock: Arc<dyn Clock>,
    ) -> Self {
        let categories = Rc::new(categories);
        let tabs = TabsWidget::new(&config.features);
        let filter = FilterWidget::new(Rc::clone(&categories), tabs.current().filter_target());
        Self {
            shell: ShellComponent::new(theme),
            login: LoginComponent::new(authentication_required),
            tabs,
            filter,
            feeds: FeedsComponent::new(config, Rc::clone(&categories)),
            reader: ReaderComponent::default(),
            subscribe: subscribe::SubscribeForm::new(Rc::clone(&categories)),
            gh: GhComponent::new(
                config.features.enable_gh_notification,
                Rc::clone(&categories),
            ),
            unsubscribe: UnsubscribePopup::default(),
            categories,
            clock,
        }
    }
}

impl tuiframework::Components for Screens {
    type Id = ComponentId;
    type Layer = Layer;
    type Keymap = Keymap;
    type OverlayId = super::overlay::OverlayId;
    type StackId = super::overlay::StackId;
    type Command = Command;
    type Event = Event;
    type Operation = Operation;
    type RenderContext<'a> = RenderContext<'a>;

    fn ids(&self) -> impl IntoIterator<Item = ComponentId> {
        ComponentId::iter()
    }

    fn get(&self, id: ComponentId) -> Option<&dyn Component<Self>> {
        Some(match id {
            ComponentId::Shell => &self.shell,
            ComponentId::Tabs => &self.tabs,
            ComponentId::Login => &self.login,
            ComponentId::Filter => &self.filter,
            ComponentId::Entries => &self.feeds.entries,
            ComponentId::Reader => &self.reader,
            ComponentId::Feeds => &self.feeds,
            ComponentId::Subscribe => &self.subscribe,
            ComponentId::GhNotifications => &self.gh,
            ComponentId::UnsubscribePopup => &self.unsubscribe,
            ComponentId::GhFilterPopup => &self.gh.notifications.filter_popup,
        })
    }

    fn get_mut(&mut self, id: ComponentId) -> Option<&mut dyn Component<Self>> {
        Some(match id {
            ComponentId::Shell => &mut self.shell,
            ComponentId::Tabs => &mut self.tabs,
            ComponentId::Login => &mut self.login,
            ComponentId::Filter => &mut self.filter,
            ComponentId::Entries => &mut self.feeds.entries,
            ComponentId::Reader => &mut self.reader,
            ComponentId::Feeds => &mut self.feeds,
            ComponentId::Subscribe => &mut self.subscribe,
            ComponentId::GhNotifications => &mut self.gh,
            ComponentId::UnsubscribePopup => &mut self.unsubscribe,
            ComponentId::GhFilterPopup => &mut self.gh.notifications.filter_popup,
        })
    }
}
