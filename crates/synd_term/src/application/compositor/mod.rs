use super::ComponentId;
use crate::event::EventKind;
use chrono::{DateTime, Utc};
use crossterm::event::{Event as CrosstermEvent, KeyEventKind};
use ratatui::Frame;
use std::{collections::HashMap, rc::Rc};
use strum::IntoEnumIterator as _;
use tracing::debug;

use crate::{
    config::Categories,
    keymap::Keymap,
    operation::Operations,
    ui::{self, theme::Theme},
};

use super::{
    Config,
    component::{Components, FrameLayout, RenderContext},
    overlay::Overlay,
};

mod commands;
mod events;
mod keymap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TerminalFocus {
    Gained,
    Lost,
}

pub(super) struct Compositor {
    pub(super) components: Components,
    subscribers: HashMap<EventKind, Vec<ComponentId>>,
    categories: Rc<Categories>,
    overlay: Overlay,
    keymap: Keymap,
    focus: TerminalFocus,
}

impl Compositor {
    pub(super) fn new(
        config: Config,
        theme: Theme,
        categories: Categories,
        authentication_required: bool,
    ) -> Self {
        let categories = Rc::new(categories);
        let components = Components::new(&config, &categories, theme, authentication_required);
        let mut subscribers = HashMap::<_, Vec<_>>::new();
        for id in ComponentId::iter() {
            for &kind in components.get(id).subscriptions() {
                subscribers.entry(kind).or_default().push(id);
            }
        }
        Self {
            components,
            subscribers,
            categories,
            overlay: Overlay::default(),
            keymap: Keymap::new(config.keymaps),
            focus: TerminalFocus::Gained,
        }
    }

    pub(super) fn quit(&mut self) {
        self.components.shell.should_quit = true;
    }

    pub(super) fn should_quit(&self) -> bool {
        self.components.shell.should_quit
    }

    pub(super) fn tick(&mut self) {
        self.components.shell.tick();
    }

    pub(super) fn is_busy(&self) -> bool {
        self.components.shell.is_busy()
    }

    pub(super) fn render(&self, frame: &mut Frame, now: DateTime<Utc>) {
        let components = &self.components;
        let cx = ui::Context {
            theme: &components.shell.theme,
            categories: &self.categories,
            focus: self.focus,
            now,
        };
        let tab = components.visible_tab();
        let cx = RenderContext {
            ui: cx,
            keymap: &self.keymap,
            layers: self.key_layers(),
            frame: FrameLayout::new(frame.area(), tab),
            filter_target: components.tabs.current().filter_target(),
            gh_options: components.gh.notifications.filter_options(),
        };
        for id in self.overlay.visible(tab) {
            components.get(id).render(frame.buffer_mut(), &cx);
        }
    }

    pub(super) fn apply_terminal_event(&mut self, event: &CrosstermEvent) -> Operations {
        match event {
            CrosstermEvent::FocusGained => self.focus = TerminalFocus::Gained,
            CrosstermEvent::FocusLost => self.focus = TerminalFocus::Lost,
            CrosstermEvent::Key(key) if key.kind != KeyEventKind::Release => {
                debug!("Handle key event: {key:?}");
                self.components.shell.dismiss_error();
                return self
                    .resolve_keymap(*key)
                    .map_or(Operations::Nop, |command| self.apply_command(command));
            }
            _ => {}
        }
        Operations::Nop
    }
}

#[cfg(test)]
mod tests;
