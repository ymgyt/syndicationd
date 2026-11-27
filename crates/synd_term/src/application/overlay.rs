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

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum OverlayId {
    Feeds,
    GitHub,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum StackId {
    Entries,
}

pub(super) fn layout() -> tuiframework::Overlay<super::component::Screens> {
    use ComponentId::{Entries, Feeds, Filter, GhNotifications, Login, Shell, Tabs};
    use tuiframework::Overlay;
    let entries = Overlay::new()
        .component(Tabs)
        .component(Entries)
        .component(Filter);
    let feeds = Overlay::named(OverlayId::Feeds)
        .component(Tabs)
        .component(Feeds)
        .component(Filter);
    let github = Overlay::named(OverlayId::GitHub)
        .component(Tabs)
        .component(GhNotifications)
        .component(Filter);
    Overlay::new().component(Shell).switch(
        super::component::Screens::visible_tab,
        [
            (None, Overlay::new().component(Login)),
            (
                Some(Tab::Entries),
                Overlay::new().stack(StackId::Entries, entries),
            ),
            (Some(Tab::Feeds), feeds),
            (Some(Tab::Gh), github),
        ],
    )
}
