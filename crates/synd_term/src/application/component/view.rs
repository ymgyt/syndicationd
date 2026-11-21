use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    widgets::{Block, Widget},
};

use super::{AuthenticationState, Components, ShellComponent};
use crate::{
    application::ComponentId,
    keymap::{Layer, LayerStack},
    ui::{
        Context,
        widgets::{
            authentication,
            entries::EntriesWidget,
            filter::{FilterContext, FilterWidget},
            gh_notifications::{FilterPopup, GhNotificationFilterOptions, GhNotificationsWidget},
            subscription::{SubscriptionWidget, UnsubscribePopup},
            tabs::{Tab, TabsWidget},
        },
    },
};

pub(in crate::application) trait Component {
    fn render(&self, buf: &mut Buffer, cx: &RenderContext<'_>);
    fn key_layers(&self) -> &[Layer];
    fn mask_lower_layers(&self, _layers: &mut LayerStack) {}
}

impl Components {
    pub(in crate::application) fn get(&self, id: ComponentId) -> &dyn Component {
        match id {
            ComponentId::Shell => &self.shell,
            ComponentId::Tabs => &self.shell.tabs,
            ComponentId::Login => self.shell.authentication(),
            ComponentId::Filter => &self.shell.filter,
            ComponentId::Entries => &self.feeds.entries,
            ComponentId::Feeds => &self.feeds.subscription,
            ComponentId::GhNotifications => &self.gh.notifications,
            ComponentId::UnsubscribePopup => &self.feeds.subscription.unsubscribe_popup,
            ComponentId::GhFilterPopup => &self.gh.notifications.filter_popup,
        }
    }
}

pub(in crate::application) struct FrameLayout {
    area: Rect,
    tabs: Rect,
    filter: Rect,
    content: Rect,
    status: Rect,
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
    pub(in crate::application) frame: FrameLayout,
    pub(in crate::application) shell: &'a ShellComponent,
    pub(in crate::application) gh_options: &'a GhNotificationFilterOptions,
}

impl Component for ShellComponent {
    fn render(&self, buf: &mut Buffer, cx: &RenderContext<'_>) {
        Block::new()
            .style(cx.ui.theme.base)
            .render(cx.frame.area, buf);
        self.prompt
            .render(cx.frame.status, buf, &cx.ui, self.visible_tab());
    }

    fn key_layers(&self) -> &[Layer] {
        &[Layer::App, Layer::Global]
    }
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

impl Component for SubscriptionWidget {
    fn render(&self, buf: &mut Buffer, cx: &RenderContext<'_>) {
        self.render(cx.frame.content, buf, &cx.ui);
    }

    fn key_layers(&self) -> &[Layer] {
        &[Layer::List, Layer::Feeds]
    }
}

impl Component for GhNotificationsWidget {
    fn render(&self, buf: &mut Buffer, cx: &RenderContext<'_>) {
        self.render(cx.frame.content, buf, &cx.ui);
    }

    fn key_layers(&self) -> &[Layer] {
        &[Layer::List, Layer::GhNotifications]
    }
}

impl Component for FilterWidget {
    fn render(&self, buf: &mut Buffer, cx: &RenderContext<'_>) {
        self.render(
            cx.frame.filter,
            buf,
            &FilterContext {
                ui: &cx.ui,
                gh_options: cx.gh_options,
                target: cx.shell.current_filter_target(),
            },
        );
    }

    fn key_layers(&self) -> &[Layer] {
        if self.is_category_filtering_active() {
            &[Layer::Filter, Layer::CategoryFilter]
        } else if self.is_search_active() {
            &[Layer::Filter, Layer::SearchPrompt]
        } else {
            &[Layer::Filter]
        }
    }
}

impl Component for UnsubscribePopup {
    fn render(&self, buf: &mut Buffer, cx: &RenderContext<'_>) {
        self.render(cx.frame.content, buf, &cx.ui);
    }

    fn key_layers(&self) -> &[Layer] {
        &[Layer::UnsubscribePopup]
    }

    fn mask_lower_layers(&self, layers: &mut LayerStack) {
        layers.cover(&[Layer::List, Layer::Feeds]);
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
