use super::RenderContext;
use super::Ui;
use crate::{
    application::{Populate, effects::Effects},
    command::{FilterCommand, FilterTarget},
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
        command: FilterCommand,
        current: FilterTarget,
    ) -> Effects {
        let (target, changed) = match command {
            FilterCommand::MoveFilterRequirement(direction) if current == FilterTarget::Feeds => {
                (current, self.move_requirement(direction))
            }
            FilterCommand::ActivateCategoryFiltering => {
                self.activate_category_filtering(current);
                return Effects::NONE;
            }
            FilterCommand::ActivateSearchFiltering => {
                self.activate_search_filtering();
                return Effects::NONE;
            }
            FilterCommand::DeactivateFiltering => {
                self.deactivate_filtering();
                return Effects::NONE;
            }
            FilterCommand::PromptInsertChar(ch) if self.is_search_active() => {
                self.insert_prompt_char(ch);
                (current, true)
            }
            FilterCommand::PromptDeleteBackward if self.is_search_active() => {
                (current, self.delete_prompt_backward())
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
            _ => return Effects::NONE,
        };
        if changed {
            self.changed(target).into()
        } else {
            Effects::NONE
        }
    }
}

impl Component<Ui> for FilterWidget {
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
            EventKind::FeedCategoriesChanged,
            EventKind::GhCategoriesChanged,
        ]
        .contains(&EventKind::from(event))
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
