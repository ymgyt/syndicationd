use crate::{
    auth::AuthenticationProvider,
    ui::components::{
        authentication::Authentication, entries::Entries, filter::Filter, status::StatusLine,
        subscription::Subscription, tabs::Tabs,
    },
};

pub mod authentication;
pub mod entries;
pub mod filter;
pub mod root;
pub mod status;
pub mod subscription;
pub mod tabs;

pub struct Components {
    pub tabs: Tabs,
    pub filter: Filter,
    pub prompt: StatusLine,
    pub subscription: Subscription,
    pub entries: Entries,
    pub auth: Authentication,
}

impl Components {
    pub fn new() -> Self {
        Self {
            tabs: Tabs::new(),
            filter: Filter::new(),
            prompt: StatusLine::new(),
            subscription: Subscription::new(),
            entries: Entries::new(),
            auth: Authentication::new(vec![
                AuthenticationProvider::Github,
                AuthenticationProvider::Google,
            ]),
        }
    }
}
