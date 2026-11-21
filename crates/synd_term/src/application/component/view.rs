use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
};

use super::Component;
use super::{AuthenticationState, ShellComponent};
use crate::{
    keymap::{Keymap, Layer, LayerStack},
    ui::{
        Context,
        widgets::{
            authentication,
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

pub(in crate::application) struct RenderContext<'a> {
    pub(in crate::application) ui: Context<'a>,
    pub(in crate::application) keymap: &'a Keymap,
    pub(in crate::application) layers: LayerStack,
    pub(in crate::application) frame: FrameLayout,
    pub(in crate::application) shell: &'a ShellComponent,
    pub(in crate::application) gh_options: &'a GhNotificationFilterOptions,
}

impl Component for AuthenticationState {
    fn render(&self, buf: &mut Buffer, cx: &RenderContext<'_>) {
        match self {
            Self::Required | Self::RequestingDeviceFlow { .. } => authentication::render_login(
                cx.frame.content,
                buf,
                cx.ui.theme,
                cx.shell.authentication_providers(),
                cx.shell.selected_authentication_provider_index(),
            ),
            Self::DeviceFlow {
                verification_url,
                user_code,
                ..
            } => authentication::render_device_flow(
                cx.frame.content,
                buf,
                cx.ui.theme,
                verification_url.as_str(),
                user_code,
            ),
            Self::NotRequired | Self::Authenticated => {}
        }
    }

    fn key_layers(&self) -> &[Layer] {
        &[Layer::Login]
    }
}

impl Component for TabsWidget {
    fn render(&self, buf: &mut Buffer, cx: &RenderContext<'_>) {
        self.render(cx.frame.tabs, buf, &cx.ui);
    }

    fn key_layers(&self) -> &[Layer] {
        &[Layer::Tabs]
    }
}

impl Component for EntriesWidget {
    fn render(&self, buf: &mut Buffer, cx: &RenderContext<'_>) {
        self.render(cx.frame.content, buf, &cx.ui);
    }

    fn key_layers(&self) -> &[Layer] {
        &[Layer::List, Layer::Entries]
    }
}

impl Component for FilterPopup {
    fn render(&self, buf: &mut Buffer, cx: &RenderContext<'_>) {
        self.render_popup(cx.frame.content, buf, &cx.ui, cx.gh_options);
    }

    fn key_layers(&self) -> &[Layer] {
        &[Layer::GhNotificationFilterPopup]
    }

    fn mask_lower_layers(&self, layers: &mut LayerStack) {
        layers.truncate_after(Layer::Tabs);
    }
}
