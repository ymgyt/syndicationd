use crate::{
    application::{
        Features,
        state::{Should, State},
    },
    config::Categories,
    ui::theme::Theme,
};

mod commands;
mod events;
mod feeds;
mod github;
mod shell;

pub(crate) use feeds::FeedsComponent;
pub(crate) use github::GitHubComponent;
pub(crate) use shell::ShellComponent;

/// Top-level application state machine that coordinates child components.
pub(crate) struct AppComponent {
    pub(crate) shell: ShellComponent,
    pub(crate) feeds: FeedsComponent,
    pub(crate) github: GitHubComponent,
}

impl AppComponent {
    pub(super) fn new(
        features: &Features,
        theme: Theme,
        categories: Categories,
        dry_run: bool,
    ) -> Self {
        let mut state = State::new();
        if dry_run {
            state.flags = Should::Quit;
        }

        Self {
            shell: ShellComponent::new(features, theme, categories, state),
            feeds: FeedsComponent::new(),
            github: GitHubComponent::new(),
        }
    }

    pub(in crate::application) fn is_feed_unsubscription_popup_open(&self) -> bool {
        self.feeds.is_unsubscribe_popup_open()
    }
}
