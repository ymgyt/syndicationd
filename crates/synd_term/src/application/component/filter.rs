use crate::{
    application::compositor::Effects,
    command::{FilterCommand, FilterTarget},
    event::Event,
    keymap::{Keymap, Layer, LayerKeymap},
};

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
            Event::FilterChanged(target).into()
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
