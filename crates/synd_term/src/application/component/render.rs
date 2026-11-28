use crate::application::component::Screens;
use ratatui::layout::{Constraint, Layout, Rect};
use tuiframework::{Component, RenderInputs};

use crate::{
    application::{Direction, effects::Effects},
    command::{Command, FilterTarget},
    event::Event,
    keymap::{Keymap, Layer, LayerStack},
    ui::{
        Context,
        widgets::{
            Dialog,
            entries::EntriesWidget,
            gh_notifications::{FilterPopup, GhNotificationFilterOptions},
            tabs::{Tab, TabsWidget},
        },
    },
};

pub(in crate::application) struct FrameLayout {
    pub(super) area: Rect,
    tabs: Rect,
    pub(super) filter: Rect,
    pub(super) content: Rect,
    pub(super) status: Rect,
}

impl FrameLayout {
    pub(super) fn dialog(&self) -> Dialog {
        let screen = self.full_screen_content();
        // Keep the tab visible while there is room above the dialog.
        let tab_height = if screen.height >= 8 {
            self.tabs.height
        } else {
            0
        };
        Dialog::new(
            screen,
            Rect {
                y: screen.y + tab_height,
                height: screen.height - tab_height,
                ..screen
            },
        )
    }

    pub(super) fn full_screen_content(&self) -> Rect {
        Rect {
            height: self.status.y.saturating_sub(self.area.y),
            ..self.area
        }
    }

    pub(in crate::application) fn new(area: Rect, tab: Option<Tab>) -> Self {
        let height = |n| Constraint::Length(if tab.is_some() { n } else { 0 });
        let [tabs, filter, content, status] = Layout::vertical([
            height(1),
            height(2),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .areas(area);
        Self {
            area,
            tabs,
            filter,
            content,
            status,
        }
    }
}

pub(crate) struct RenderContext<'a> {
    pub(in crate::application) ui: Context<'a>,
    pub(in crate::application) keymap: &'a Keymap,
    pub(in crate::application) layers: &'a LayerStack,
    pub(in crate::application) frame: FrameLayout,
    pub(in crate::application) filter_target: FilterTarget,
    pub(in crate::application) gh_options: &'a GhNotificationFilterOptions,
}

impl<'a> From<RenderInputs<'a, Screens>> for RenderContext<'a> {
    fn from(cx: RenderInputs<'a, Screens>) -> Self {
        let ui = cx.components;
        Self {
            ui: Context {
                theme: &ui.shell.theme,
                categories: &ui.categories,
                focus: cx.focus,
                now: ui.clock.now(),
            },
            keymap: cx.keymap,
            layers: cx.layers,
            frame: FrameLayout::new(cx.area, ui.visible_tab()),
            filter_target: ui.tabs.current().filter_target(),
            gh_options: ui.gh.notifications.filter_options(),
        }
    }
}

impl TabsWidget {
    pub(super) fn move_tab(&mut self, direction: Direction, effects: &mut Effects) {
        let previous = self.current();
        let tab = self.move_selection(direction);
        if tab != previous {
            effects.emit(Event::TabChanged(tab));
        }
    }
}

impl Component<Screens> for TabsWidget {
    fn apply_command(&mut self, command: Command, effects: &mut Effects) {
        if let Command::MoveTab(action) = command {
            self.move_tab(action, effects);
        }
    }

    fn render(&self, frame: &mut ratatui::Frame<'_>, cx: &RenderContext<'_>) {
        let buf = frame.buffer_mut();
        self.render(cx.frame.tabs, buf, &cx.ui);
    }

    fn key_layers(&self) -> &[Layer] {
        &[Layer::Tabs]
    }
}

impl Component<Screens> for EntriesWidget {
    fn apply_command(&mut self, command: Command, _: &mut Effects) {
        if let Command::List(action) = command {
            self.apply_list(action);
        }
    }

    fn render(&self, frame: &mut ratatui::Frame<'_>, cx: &RenderContext<'_>) {
        let buf = frame.buffer_mut();
        self.render(cx.frame.content, buf, &cx.ui);
    }

    fn key_layers(&self) -> &[Layer] {
        &[Layer::List, Layer::Entries]
    }
}

impl Component<Screens> for FilterPopup {
    fn apply_command(&mut self, command: Command, effects: &mut Effects) {
        if let Command::GhFilterPopup(action) = command {
            Self::apply_command(action, effects);
        }
    }

    fn render(&self, frame: &mut ratatui::Frame<'_>, cx: &RenderContext<'_>) {
        let buf = frame.buffer_mut();
        self.render_popup(cx.frame.content, buf, &cx.ui, cx.gh_options);
    }

    fn key_layers(&self) -> &[Layer] {
        &[Layer::GhNotificationFilterPopup]
    }

    fn mask_lower_layers(&self, layers: &mut LayerStack) {
        layers.truncate_after(Layer::Tabs);
    }
}
