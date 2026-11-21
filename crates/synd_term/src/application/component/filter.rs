use super::{Component, RenderContext};
use crate::{
    application::Populate,
    event::{Event, EventKind},
    ui::widgets::filter::{FilterContext, FilterWidget},
};
use crate::{
    application::effects::Effects,
    command::{FilterCommand, FilterTarget},
    keymap::{Keymap, Layer, LayerKeymap},
};
use ratatui::buffer::Buffer;

use super::ShellComponent;

impl ShellComponent {
    pub(in crate::application) fn apply_filter_command(
        &mut self,
        command: FilterCommand,
    ) -> Effects {
        let current = self.current_filter_target();
        let (target, changed) = match command {
            FilterCommand::MoveFilterRequirement(direction) if current == FilterTarget::Feeds => {
                (current, self.filter.move_requirement(direction))
            }
            FilterCommand::ActivateCategoryFiltering => {
                self.filter.activate_category_filtering(current);
                return Effects::NONE;
            }
            FilterCommand::ActivateSearchFiltering => {
                self.filter.activate_search_filtering();
                return Effects::NONE;
            }
            FilterCommand::DeactivateFiltering => {
                self.filter.deactivate_filtering();
                return Effects::NONE;
            }
            FilterCommand::PromptInsertChar(ch) if self.filter.is_search_active() => {
                self.filter.insert_prompt_char(ch);
                (current, true)
            }
            FilterCommand::PromptDeleteBackward if self.filter.is_search_active() => {
                (current, self.filter.delete_prompt_backward())
            }
            FilterCommand::ToggleFilterCategory { category, target }
                if self.filter.category_filter_target() == Some(target) =>
            {
                (target, self.filter.toggle_category_state(&category, target))
            }
            FilterCommand::ActivateAllFilterCategories { target }
                if self.filter.category_filter_target() == Some(target) =>
            {
                (target, self.filter.set_all_categories_active(target, true))
            }
            FilterCommand::DeactivateAllFilterCategories { target }
                if self.filter.category_filter_target() == Some(target) =>
            {
                (target, self.filter.set_all_categories_active(target, false))
            }
            _ => return Effects::NONE,
        };
        if changed {
            self.filter.changed(target).into()
        } else {
            Effects::NONE
        }
    }

    pub(in crate::application) fn install_filter_keymap(&self, keymap: &mut Keymap) {
        keymap.clear_layer_keymap(Layer::CategoryFilter);
        keymap.clear_layer_keymap(Layer::SearchPrompt);
        if let Some(categories) = self.filter.category_filter_keymap() {
            keymap.set_layer_keymap(categories);
        } else if self.filter.is_search_active() {
            keymap.set_layer_keymap(LayerKeymap::search_prompt());
        }
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
    fn subscriptions(&self) -> &'static [EventKind] {
        &[
            EventKind::FeedCategoriesChanged,
            EventKind::GhCategoriesChanged,
        ]
    }

    fn apply_event(&mut self, event: &Event) -> Effects {
        match event {
            Event::FeedCategoriesChanged(categories) => {
                self.update_categories(FilterTarget::Feeds, Populate::Replace, categories);
            }
            Event::GhCategoriesChanged {
                populate,
                categories,
            } => self.update_categories(FilterTarget::GhNotifications, *populate, categories),
            _ => {}
        }
        Effects::NONE
    }
}
