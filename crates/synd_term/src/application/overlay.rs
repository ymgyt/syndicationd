use crate::ui::widgets::tabs::Tab;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ComponentId {
    Shell,
    Login,
    Tabs,
    Entries,
    Feeds,
    GhNotifications,
    Filter,
    UnsubscribePopup,
    GhFilterPopup,
}

#[derive(Default)]
pub(super) struct Overlay {
    popups: Vec<(Tab, ComponentId)>,
}

impl Overlay {
    pub(super) fn visible(
        &self,
        tab: Option<Tab>,
    ) -> impl DoubleEndedIterator<Item = ComponentId> + '_ {
        use ComponentId::{Entries, Feeds, Filter, GhNotifications, Login, Shell, Tabs};
        let base: &[ComponentId] = match tab {
            None => &[Shell, Login],
            Some(Tab::Entries) => &[Shell, Tabs, Entries, Filter],
            Some(Tab::Feeds) => &[Shell, Tabs, Feeds, Filter],
            Some(Tab::Gh) => &[Shell, Tabs, GhNotifications, Filter],
        };
        base.iter().copied().chain(
            self.popups
                .iter()
                .filter_map(move |&(owner, popup)| (Some(owner) == tab).then_some(popup)),
        )
    }

    pub(super) fn open(&mut self, tab: Tab, id: ComponentId) {
        self.close(tab, id);
        self.popups.push((tab, id));
    }

    pub(super) fn close(&mut self, tab: Tab, id: ComponentId) {
        self.popups.retain(|entry| *entry != (tab, id));
    }
}
