use crate::ui::widgets::tabs::Tab;

#[derive(Clone, Copy, Debug, Eq, PartialEq, strum::EnumIter)]
pub(crate) enum ComponentId {
    Shell,
    Login,
    Tabs,
    Entries,
    Reader,
    Feeds,
    Subscribe,
    GhNotifications,
    Filter,
    UnsubscribePopup,
    GhFilterPopup,
}

#[derive(Default)]
pub(super) struct Overlay {
    overlays: Vec<(Tab, ComponentId)>,
}

impl Overlay {
    pub(super) fn visible(
        &self,
        tab: Option<Tab>,
    ) -> impl DoubleEndedIterator<Item = ComponentId> + '_ {
        use ComponentId::{
            Entries, Feeds, Filter, GhNotifications, Login, Reader, Shell, Subscribe, Tabs,
        };
        let overlays = self
            .overlays
            .iter()
            .filter_map(move |&(owner, component)| (Some(owner) == tab).then_some(component));
        let base: &[ComponentId] = match tab {
            None => &[Shell, Login],
            Some(_) if overlays.clone().any(|id| matches!(id, Reader | Subscribe)) => &[Shell],
            Some(Tab::Entries) => &[Shell, Tabs, Entries, Filter],
            Some(Tab::Feeds) => &[Shell, Tabs, Feeds, Filter],
            Some(Tab::Gh) => &[Shell, Tabs, GhNotifications, Filter],
        };
        base.iter().copied().chain(overlays)
    }

    pub(super) fn open(&mut self, sender: ComponentId) {
        let tab = match sender {
            ComponentId::Reader => Tab::Entries,
            ComponentId::Subscribe | ComponentId::UnsubscribePopup => Tab::Feeds,
            ComponentId::GhFilterPopup => Tab::Gh,
            _ => unreachable!("cannot open component: {sender:?}"),
        };
        self.close(sender);
        self.overlays.push((tab, sender));
    }

    pub(super) fn close(&mut self, sender: ComponentId) {
        self.overlays.retain(|&(_, component)| component != sender);
    }
}
