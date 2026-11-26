use std::{rc::Rc, sync::Arc};

use strum::IntoEnumIterator as _;
use tuiframework::Component;

use super::{Clock, ComponentId, Config, effects::Effects, overlay::Overlay};
use crate::{
    command::Command,
    config::Categories,
    crossterm::event::Event as TerminalEvent,
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

/// Owns the synd screens, their shared layout inputs, and the current overlays.
pub(crate) struct Ui {
    categories: Rc<Categories>,
    clock: Arc<dyn Clock>,
    overlay: Overlay,
    pub(in crate::application) shell: ShellComponent,
    pub(in crate::application) login: LoginComponent,
    pub(in crate::application) filter: FilterWidget,
    pub(in crate::application) tabs: TabsWidget,
    pub(crate) feeds: FeedsComponent,
    reader: ReaderComponent,
    unsubscribe: UnsubscribePopup,
    pub(crate) gh: GhComponent,
}

impl Ui {
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
        Self {
            shell: ShellComponent::new(theme),
            login: LoginComponent::new(authentication_required),
            tabs: TabsWidget::new(&config.features),
            filter: FilterWidget::new(Rc::clone(&categories)),
            feeds: FeedsComponent::new(config, Rc::clone(&categories)),
            reader: ReaderComponent::default(),
            gh: GhComponent::new(
                config.features.enable_gh_notification,
                Rc::clone(&categories),
            ),
            unsubscribe: UnsubscribePopup::default(),
            overlay: Overlay::default(),
            categories,
            clock,
        }
    }

    fn get_mut(&mut self, id: ComponentId) -> &mut dyn Component<Self> {
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

impl tuiframework::Ui for Ui {
    type Id = ComponentId;
    type Layer = Layer;
    type Keymap = Keymap;
    type Theme = Theme;
    type Command = Command;
    type Event = Event;
    type Operation = Operation;
    type Context<'a> = RenderContext<'a>;

    fn theme(&self) -> &Self::Theme {
        &self.shell.theme
    }

    fn ids(&self) -> impl IntoIterator<Item = ComponentId> {
        ComponentId::iter()
    }

    fn component(&self, id: ComponentId) -> Option<&dyn Component<Self>> {
        Some(match id {
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
        })
    }

    fn component_mut(&mut self, id: ComponentId) -> Option<&mut dyn Component<Self>> {
        Some(self.get_mut(id))
    }

    fn visible(&self) -> impl IntoIterator<Item = ComponentId> {
        self.overlay.visible(self.visible_tab())
    }

    fn apply_command(&mut self, to: ComponentId, command: Command) -> Effects {
        match command {
            Command::Open => self.overlay.open(to),
            Command::Close => self.overlay.close(to),
            Command::Filter(command) => {
                let target = self.tabs.current().filter_target();
                return self.filter.apply_command(command, target);
            }
            _ => return self.get_mut(to).apply_command(command),
        }
        Effects::NONE
    }

    fn update_keymap(&mut self, map: &mut Keymap) {
        self.filter.install_filter_keymap(map);
    }

    fn on_input(&mut self, event: &TerminalEvent) -> Effects {
        if matches!(event, TerminalEvent::Key(_)) {
            self.shell.dismiss_error();
        }
        Effects::NONE
    }

    fn tick(&mut self) {
        self.shell.tick();
    }

    fn is_busy(&self) -> bool {
        self.shell.is_busy()
    }
}
