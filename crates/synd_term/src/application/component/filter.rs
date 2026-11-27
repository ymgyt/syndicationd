use super::RenderContext;
use super::Screens;
use crate::{
    application::{Populate, effects::Effects},
    command::{Command, FilterCommand, FilterTarget},
    event::{Event, EventKind},
    keymap::Layer,
    ui::widgets::{
        filter::{FilterContext, FilterWidget},
        tabs::Tab,
    },
};
use tuiframework::Component;

impl Tab {
    pub(in crate::application) fn filter_target(self) -> FilterTarget {
        match self {
            Self::Feeds | Self::Entries => FilterTarget::Feeds,
            Self::Gh => FilterTarget::GhNotifications,
        }
    }
}

impl FilterWidget {
    pub(in crate::application) fn apply_command(
        &mut self,
        command: Command,
        current: FilterTarget,
        effects: &mut Effects,
    ) {
        let command = match command {
            Command::Filter(command) => command,
            Command::Prompt(action) if self.is_search_active() && self.edit_prompt(&action) => {
                return effects.emit(self.changed(current));
            }
            _ => return,
        };
        let (target, changed) = match command {
            FilterCommand::MoveFilterRequirement(direction) if current == FilterTarget::Feeds => {
                (current, self.move_requirement(direction))
            }
            FilterCommand::ActivateCategoryFiltering => {
                self.activate_category_filtering(current);
                return;
            }
            FilterCommand::ActivateSearchFiltering => {
                self.activate_search_filtering();
                return;
            }
            FilterCommand::DeactivateFiltering => {
                self.deactivate_filtering();
                return;
            }
            FilterCommand::ToggleFilterCategory { category, target }
                if self.category_filter_target() == Some(target) =>
            {
                (target, self.toggle_category_state(&category, target))
            }
            FilterCommand::ActivateAllFilterCategories { target }
                if self.category_filter_target() == Some(target) =>
            {
                (target, self.set_all_categories_active(target, true))
            }
            FilterCommand::DeactivateAllFilterCategories { target }
                if self.category_filter_target() == Some(target) =>
            {
                (target, self.set_all_categories_active(target, false))
            }
            _ => return,
        };
        if changed {
            effects.emit(self.changed(target));
        }
    }
}

impl Component<Screens> for FilterWidget {
    fn apply_command(&mut self, command: Command, effects: &mut Effects) {
        self.apply_command(command, self.target, effects);
    }
    fn update_keymap(&mut self, map: &mut crate::keymap::Keymap) {
        self.install_filter_keymap(map);
    }
    fn on_input(
        &mut self,
        input: &crate::crossterm::event::Event,
        frontmost: bool,
        effects: &mut Effects,
    ) {
        if frontmost && let crate::crossterm::event::Event::Paste(text) = input {
            self.apply_command(
                Command::Prompt(crate::command::PromptAction::Paste(text.clone())),
                self.target,
                effects,
            );
        }
    }
    fn render(&self, frame: &mut ratatui::Frame<'_>, cx: &RenderContext<'_>) {
        let buf = frame.buffer_mut();
        self.render(
            cx.frame.filter,
            buf,
            &FilterContext {
                ui: &cx.ui,
                gh_options: cx.gh_options,
                target: cx.filter_target,
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
    fn subscribes(&self, event: &Event) -> bool {
        [
            EventKind::TabChanged,
            EventKind::FeedCategoriesChanged,
            EventKind::GhCategoriesChanged,
        ]
        .contains(&EventKind::from(event))
    }

    fn apply_event(&mut self, event: &Event, _: &mut Effects) {
        match event {
            Event::TabChanged(tab) => self.target = tab.filter_target(),
            Event::FeedCategoriesChanged(categories) => {
                self.update_categories(FilterTarget::Feeds, Populate::Replace, categories);
            }
            Event::GhCategoriesChanged {
                populate,
                categories,
            } => self.update_categories(FilterTarget::GhNotifications, *populate, categories),
            _ => {}
        }
    }
}
