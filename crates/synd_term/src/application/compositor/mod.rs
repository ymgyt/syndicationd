use chrono::{DateTime, Utc};
use crossterm::event::{Event as CrosstermEvent, KeyEventKind};
use ratatui::Frame;
use tracing::debug;

use crate::{
    config::Categories,
    keymap::Keymap,
    operation::Operations,
    ui::{self, theme::Theme},
};

use super::{
    Config, Features,
    component::{AuthenticationState, Components, FrameLayout, RenderContext},
    overlay::Overlay,
};

mod commands;
mod events;
mod keymap;
pub(in crate::application) use commands::Effects;

pub(super) struct Compositor {
    pub(super) components: Components,
    overlay: Overlay,
    keymap: Keymap,
    pub(super) features: Features,
    entries_limit: usize,
    feeds_per_pagination: i64,
}

impl Compositor {
    pub(super) fn new(
        config: Config,
        theme: Theme,
        categories: Categories,
        authentication: AuthenticationState,
    ) -> Self {
        Self {
            components: Components::new(&config.features, theme, categories, authentication),
            overlay: Overlay::default(),
            keymap: Keymap::new(config.keymaps),
            features: config.features,
            entries_limit: config.entries_limit,
            feeds_per_pagination: config.feeds_per_pagination,
        }
    }

    pub(super) fn is_busy(&self) -> bool {
        !self.components.shell.in_flight.is_empty()
    }

    pub(super) fn render(&self, frame: &mut Frame, now: DateTime<Utc>) {
        let components = &self.components;
        let cx = ui::Context {
            theme: &components.shell.theme,
            in_flight: &components.shell.in_flight,
            categories: &components.shell.categories,
            focus: components.shell.focus(),
            now,
        };
        let tab = components.shell.visible_tab();
        let cx = RenderContext {
            ui: cx,
            frame: FrameLayout::new(frame.area(), tab),
            shell: &components.shell,
            gh_options: components.gh.notifications.filter_options(),
        };
        for id in self.overlay.visible(tab) {
            components.get(id).render(frame.buffer_mut(), &cx);
        }
    }

    /// Starts the first remote synchronization after API access becomes available.
    pub(super) fn bootstrap(&mut self) -> Operations {
        let feeds: Operations = self
            .components
            .feeds
            .bootstrap(self.feeds_per_pagination, self.entries_limit)
            .into();
        let gh: Operations = if self.features.enable_gh_notification {
            self.components.gh.bootstrap().into()
        } else {
            Operations::Nop
        };
        [feeds, gh].into()
    }

    pub(super) fn apply_terminal_event(&mut self, event: &CrosstermEvent) -> Operations {
        match event {
            CrosstermEvent::FocusGained => self.components.shell.focus_gained(),
            CrosstermEvent::FocusLost => self.components.shell.focus_lost(),
            CrosstermEvent::Key(key) if key.kind != KeyEventKind::Release => {
                debug!("Handle key event: {key:?}");
                self.components.shell.prompt.clear_error_message();
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
