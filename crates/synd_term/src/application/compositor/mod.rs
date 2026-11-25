pub(crate) use super::component::SyndUi;
use super::{
    ComponentId,
    component::{FrameLayout, RenderContext},
};
use crate::{
    command::Command,
    event::Event,
    keymap::{Keymap, KeymapAction, Layer},
    operation::Operation,
    ui,
};
use strum::IntoEnumIterator as _;
use tuiframework::crossterm::event::Event as TerminalEvent;
use tuiframework::{Component, Components as ComponentStore, Effects, KeyAction};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TerminalFocus {
    Gained,
    Lost,
}
impl KeyAction<SyndUi> for KeymapAction {
    fn command_for(self, owner: ComponentId) -> (ComponentId, Command) {
        let command = Command::from(self);
        (command.target(owner), command)
    }
}
impl ComponentStore for SyndUi {
    type Id = ComponentId;
    type Layer = Layer;
    type Keymap = Keymap;
    type Theme = ui::theme::Theme;
    fn theme(&self) -> &Self::Theme {
        &self.shell.theme
    }
    type Command = Command;
    type Event = Event;
    type Operation = Operation;
    type View<'a> = RenderContext<'a>;
    fn ids(&self) -> impl IntoIterator<Item = ComponentId> {
        ComponentId::iter()
    }
    fn component(&self, id: ComponentId) -> Option<&dyn Component<Self>> {
        Some(self.get(id))
    }
    fn component_mut(&mut self, id: ComponentId) -> Option<&mut dyn Component<Self>> {
        Some(self.get_mut(id))
    }
    fn visible(&self) -> impl IntoIterator<Item = ComponentId> {
        self.overlay.visible(self.visible_tab())
    }
    fn command(&mut self, to: ComponentId, command: Command) -> Effects<Self> {
        match command {
            Command::Open => self.overlay.open(to),
            Command::Close => self.overlay.close(to),
            Command::Filter(command) => {
                let target = self.tabs.current().filter_target();
                return self
                    .filter
                    .apply_command(command, target)
                    .into_framework(to);
            }
            _ => return self.get_mut(to).command(command),
        }
        Effects::NONE
    }
    fn start(&mut self) -> Effects<Self> {
        if self.login.has_api_access() {
            Effects::NONE.emit(Event::ApiAccessReady)
        } else {
            Effects::NONE
        }
    }

    fn update_keymap(&mut self, map: &mut Keymap) {
        self.filter.install_filter_keymap(map);
    }
    fn on_input(&mut self, event: &TerminalEvent) -> Effects<Self> {
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
impl<'a> From<tuiframework::RenderContext<'a, SyndUi>> for RenderContext<'a> {
    fn from(cx: tuiframework::RenderContext<'a, SyndUi>) -> Self {
        let components = cx.components;
        Self {
            ui: ui::Context {
                theme: cx.theme,
                categories: &components.categories,
                focus: if cx.focused {
                    TerminalFocus::Gained
                } else {
                    TerminalFocus::Lost
                },
                now: components.clock.now(),
            },
            keymap: cx.keymap,
            layers: cx.layers,
            frame: FrameLayout::new(cx.area, components.visible_tab()),
            filter_target: components.tabs.current().filter_target(),
            gh_options: components.gh.notifications.filter_options(),
        }
    }
}
#[cfg(test)]
mod tests;
