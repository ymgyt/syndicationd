use crate::ui::components::filter::category::CategoriesState;

#[derive(Debug)]
pub(super) struct GhNotificationHandler {
    pub(super) categories_state: CategoriesState,
}

impl GhNotificationHandler {
    pub(crate) fn new() -> Self {
        Self {
            categories_state: CategoriesState::new(),
        }
    }
}
