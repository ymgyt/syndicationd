use crate::application::{ComponentId, compositor::SyndUi};
use ratatui::layout::{Constraint, Layout, Rect};
use tuiframework::{Component, Effects as UiEffects};

use crate::{
    application::{Direction, effects::Effects},
    command::{Command, FilterTarget},
    event::Event,
    keymap::{Keymap, Layer, LayerStack},
    ui::{
        Context,
        widgets::{
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

impl TabsWidget {
    pub(super) fn move_tab(&mut self, direction: Direction) -> Effects {
        let previous = self.current();
        let tab = self.move_selection(direction);
        if tab == previous {
            Effects::NONE
        } else {
            Event::TabChanged(tab).into()
        }
    }
}

impl Component<SyndUi> for TabsWidget {
    fn command(&mut self, command: Command) -> UiEffects<SyndUi> {
        match command {
            Command::MoveTab(action) => self.move_tab(action).into_framework(ComponentId::Tabs),
            _ => UiEffects::NONE,
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

impl Component<SyndUi> for EntriesWidget {
    fn command(&mut self, command: Command) -> UiEffects<SyndUi> {
        match command {
            Command::List(action) => {
                self.apply_list(action);
                UiEffects::NONE
            }
            _ => UiEffects::NONE,
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

impl Component<SyndUi> for FilterPopup {
    fn command(&mut self, command: Command) -> UiEffects<SyndUi> {
        match command {
            Command::GhFilterPopup(action) => {
                Self::apply_command(action).into_framework(ComponentId::GhFilterPopup)
            }
            _ => UiEffects::NONE,
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
