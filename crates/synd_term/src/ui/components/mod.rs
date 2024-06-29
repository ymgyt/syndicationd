use crate::{
    application::Features,
    auth::AuthenticationProvider,
    ui::components::{
        authentication::Authentication, entries::Entries, filter::Filter,
        gh_notifications::GhNotifications, status::StatusLine, subscription::Subscription,
        tabs::Tabs,
    },
};

pub(crate) mod authentication;
pub(crate) mod entries;
pub(crate) mod filter;
pub(crate) mod gh_notifications;
pub(crate) mod root;
pub(crate) mod status;
pub(crate) mod subscription;
pub(crate) mod tabs;

mod collections;

pub(crate) struct Components {
    pub tabs: Tabs,
    pub filter: Filter,
    pub prompt: StatusLine,
    pub subscription: Subscription,
    pub entries: Entries,
    pub gh_notifications: GhNotifications,
    pub auth: Authentication,
}

impl Components {
    pub fn new(features: &'_ Features) -> Self {
        Self {
            tabs: Tabs::new(features),
            filter: Filter::new(),
            prompt: StatusLine::new(),
            subscription: Subscription::new(),
            entries: Entries::new(),
            gh_notifications: GhNotifications::new(),
            auth: Authentication::new(vec![
                AuthenticationProvider::Github,
                AuthenticationProvider::Google,
            ]),
        }
    }
}
