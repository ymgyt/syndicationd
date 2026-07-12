use crate::{
    application::{
        Direction, Features, TerminalFocus,
        state::{Should, State},
    },
    auth::AuthenticationProvider,
    config::Categories,
    operation::Operation,
    ui::{
        theme::{Palette, Theme},
        widgets::{
            authentication::{AuthWidget, AuthenticateState},
            filter::{FilterLane, FilterWidget, Filterer},
            status::StatusLineWidget,
            tabs::{Tab, TabsWidget},
        },
    },
};

/// Global terminal interaction state shared across domain components.
pub(crate) struct ShellComponent {
    pub(in crate::application) theme: Theme,
    pub(in crate::application) categories: Categories,
    state: State,
    pub(crate) tabs: TabsWidget,
    pub(crate) filter: FilterWidget,
    pub(crate) prompt: StatusLineWidget,
    pub(crate) auth: AuthWidget,
}

impl ShellComponent {
    pub(super) fn new(
        features: &Features,
        theme: Theme,
        categories: Categories,
        state: State,
    ) -> Self {
        Self {
            theme,
            categories,
            state,
            tabs: TabsWidget::new(features),
            filter: FilterWidget::new(),
            prompt: StatusLineWidget::new(),
            auth: AuthWidget::new(vec![
                AuthenticationProvider::Github,
                AuthenticationProvider::Google,
            ]),
        }
    }

    pub(in crate::application) fn quit(&mut self) {
        self.state.flags.insert(Should::Quit);
    }

    pub(in crate::application) fn should_quit(&self) -> bool {
        self.state.flags.contains(Should::Quit)
    }

    pub(in crate::application) fn clear_quit_request(&mut self) {
        self.state.flags.remove(Should::Quit);
    }

    pub(in crate::application) fn request_render(&mut self) {
        self.state.flags.insert(Should::Render);
    }

    pub(in crate::application) fn should_render(&self) -> bool {
        self.state.flags.contains(Should::Render)
    }

    pub(in crate::application) fn clear_render_request(&mut self) {
        self.state.flags.remove(Should::Render);
    }

    pub(in crate::application) fn focus(&self) -> TerminalFocus {
        self.state.focus()
    }

    pub(in crate::application) fn focus_gained(&mut self) {
        self.state.focus_gained();
    }

    pub(in crate::application) fn focus_lost(&mut self) {
        self.state.focus_lost();
    }

    pub(in crate::application) fn authenticate(&self) -> Option<Operation> {
        (self.auth.state() == &AuthenticateState::NotAuthenticated).then(|| {
            Operation::StartDeviceFlow {
                provider: self.auth.selected_provider(),
            }
        })
    }

    pub(in crate::application) fn move_authentication_provider(&mut self, direction: Direction) {
        self.auth.move_selection(direction);
    }

    pub(in crate::application) fn move_tab_selection(&mut self, direction: Direction) -> Tab {
        self.tabs.move_selection(direction)
    }

    pub(in crate::application) fn move_filter_requirement(
        &mut self,
        direction: Direction,
    ) -> Filterer {
        self.filter.move_requirement(direction)
    }

    pub(in crate::application) fn activate_category_filtering(&mut self) {
        self.filter
            .activate_category_filtering(self.tabs.current().into());
    }

    pub(in crate::application) fn active_filterer(&self) -> Filterer {
        self.filter.filterer(self.tabs.current().into())
    }

    pub(in crate::application) fn toggle_filter_category(
        &mut self,
        category: &synd_feed::types::Category<'static>,
        lane: FilterLane,
    ) -> Filterer {
        self.filter.toggle_category_state(category, lane)
    }

    pub(in crate::application) fn activate_all_filter_categories(
        &mut self,
        lane: FilterLane,
    ) -> Filterer {
        self.filter.activate_all_categories_state(lane)
    }

    pub(in crate::application) fn deactivate_all_filter_categories(
        &mut self,
        lane: FilterLane,
    ) -> Filterer {
        self.filter.deactivate_all_categories_state(lane)
    }

    pub(in crate::application) fn rotate_theme(&mut self) {
        let palette = match self.theme.name {
            "ferra" => Palette::solarized_dark(),
            "solarized_dark" => Palette::helix(),
            "helix" => Palette::dracula(),
            "dracula" => Palette::eldritch(),
            _ => Palette::ferra(),
        };
        self.theme = Theme::with_palette(palette);
    }
}
