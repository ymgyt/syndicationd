use crate::{
    auth::AuthenticationProvider,
    ui::components::{
        authentication::Authentication, entries::Entries, filter::Filter, prompt::Prompt,
        subscription::Subscription, tabs::Tabs,
    },
};

pub mod authentication;
pub mod entries;
pub mod filter;
pub mod prompt;
pub mod root;
pub mod subscription;
pub mod tabs;

pub struct Components {
    pub tabs: Tabs,
    pub filter: Filter,
    pub prompt: Prompt,
    pub subscription: Subscription,
    pub entries: Entries,
    pub auth: Authentication,
}

impl Components {
    pub fn new() -> Self {
        Self {
            tabs: Tabs::new(),
            filter: Filter::new(),
            prompt: Prompt::new(),
            subscription: Subscription::new(),
            entries: Entries::new(),
            auth: Authentication::new(vec![
                AuthenticationProvider::Github,
                AuthenticationProvider::Google,
            ]),
        }
    }
}
