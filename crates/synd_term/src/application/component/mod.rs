use crate::{application::Features, config::Categories, ui::theme::Theme};

mod commands;
mod events;
mod feeds;
mod gh;
mod shell;

pub(crate) use feeds::FeedsComponent;
pub(crate) use gh::GhComponent;
pub(crate) use shell::{ApiAccessTransition, AuthenticationState, ShellComponent};

/// Top-level application state machine that coordinates child components.
pub(crate) struct Components {
    pub(crate) shell: ShellComponent,
    pub(crate) feeds: FeedsComponent,
    pub(crate) gh: GhComponent,
}

impl Components {
    pub(super) fn new(
        features: &Features,
        theme: Theme,
        categories: Categories,
        authentication: AuthenticationState,
    ) -> Self {
        Self {
            shell: ShellComponent::new(features, theme, categories, authentication),
            feeds: FeedsComponent::new(),
            gh: GhComponent::new(),
        }
    }

    pub(in crate::application) fn is_feed_unsubscription_popup_open(&self) -> bool {
        self.feeds.is_unsubscribe_popup_open()
    }
}
