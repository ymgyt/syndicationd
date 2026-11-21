use super::*;
use crate::{
    application::{Populate, RequestId, RequestKind},
    event::{Event, GhEvent},
    key,
    operation::Operation,
    types::gh::Notification,
};

fn ui(config: Config) -> Compositor {
    Compositor::new(
        config,
        Theme::default(),
        Categories::default_toml(),
        AuthenticationState::NotRequired,
    )
}

#[test]
fn entering_feeds_requests_the_configured_page_size() {
    let mut ui = ui(Config {
        feeds_per_pagination: 17,
        ..Config::default()
    });
    assert!(matches!(
        ui.apply_terminal_event(&key!(tab)),
        Operations::One(Operation::FetchSubscription {
            populate: Populate::Replace,
            after: None,
            first: 17
        })
    ));
}

#[test]
fn shared_list_binding_follows_the_visible_list_and_yields_to_the_popup() {
    use crate::{
        application::ComponentId,
        command::{Command, ListCommand},
        keymap::{CompiledKeymaps, KeyStroke},
    };
    let bindings = toml::from_str(
        r#"
        [list]
        keymap = [{ on = "x", command = "list.last" }]
    "#,
    )
    .unwrap();
    let mut ui = ui(Config {
        features: Features {
            enable_gh_notification: true,
        },
        keymaps: CompiledKeymaps::default_with_user_config(bindings).unwrap(),
        ..Config::default()
    });
    let key = KeyStroke::from_char('x');
    for owner in [
        ComponentId::GhNotifications,
        ComponentId::Entries,
        ComponentId::Feeds,
    ] {
        assert!(matches!(ui.resolve_keymap(key.into()), Some(Command::List {
            to, command: ListCommand::Last,
        }) if to == owner));
        let _ = ui.apply_terminal_event(&key!(tab));
    }
    let _ = ui.apply_terminal_event(&key!('f'));
    assert!(ui.resolve_keymap(key.into()).is_none());
    let _ = ui.apply_terminal_event(&key!(esc));
    assert!(matches!(
        ui.resolve_keymap(key.into()),
        Some(Command::List {
            to: ComponentId::GhNotifications,
            command: ListCommand::Last,
        })
    ));
}

fn notification(id: u64) -> Notification {
    serde_json::from_value::<octocrab::models::activity::Notification>(serde_json::json!({
        "id": id.to_string(),
        "repository": { "id": 1, "name": "synd", "full_name": "test/synd", "url": "https://api.github.com/repos/test/synd" },
        "subject": { "type": "Release", "title": "Release" },
        "reason": "subscribed",
        "unread": true,
        "updated_at": "2026-01-01T00:00:00Z",
        "url": format!("https://api.github.com/notifications/threads/{id}")
    })).unwrap().into()
}

fn start_page(ui: &mut Compositor, id: RequestId, operation: Operations) -> Populate {
    let Operations::One(Operation::FetchGhNotifications {
        generation,
        populate,
        params,
    }) = operation
    else {
        panic!("expected one GitHub page request, got {operation:?}");
    };
    assert!(matches!(
        ui.apply_event(Event::RequestEmitted {
            request_id: id,
            kind: RequestKind::FetchGhNotifications {
                generation,
                page: params.page
            }
        }),
        Operations::Nop
    ));
    populate
}

fn deliver_page(
    ui: &mut Compositor,
    id: RequestId,
    populate: Populate,
    notification_id: u64,
) -> Operations {
    let operations = ui.apply_event(Event::Gh {
        request_id: id,
        event: GhEvent::NotificationsFetched {
            populate,
            notifications: vec![notification(notification_id)],
        },
    });
    assert!(matches!(
        ui.apply_event(Event::RequestCompleted {
            request_id: id,
            result: Ok(())
        }),
        Operations::Nop
    ));
    operations
}

#[test]
fn previous_lists_next_page_arriving_after_reload_is_ignored() {
    let mut ui = ui(Config {
        features: Features {
            enable_gh_notification: true,
        },
        ..Config::default()
    });
    let first = RequestId::new(1);
    let old_page = RequestId::new(2);
    let refreshed = RequestId::new(3);

    let fetch = ui.apply_terminal_event(&key!('r'));
    let populate = start_page(&mut ui, first, fetch);
    let fetch = deliver_page(&mut ui, first, populate, 1);
    let append = start_page(&mut ui, old_page, fetch);
    assert_eq!(append, Populate::Append);

    let fetch = ui.apply_terminal_event(&key!('r'));
    let replace = start_page(&mut ui, refreshed, fetch);
    assert_eq!(replace, Populate::Replace);
    let next = deliver_page(&mut ui, refreshed, replace, 2);
    assert!(
        matches!(next, Operations::One(Operation::FetchGhNotifications { params, .. }) if params.page == 2)
    );

    assert!(matches!(
        deliver_page(&mut ui, old_page, append, 3),
        Operations::Nop
    ));
    let list = &ui.components.gh.notifications;
    assert_eq!(list.notification_ids(), vec![2u64.into()]);
    assert_eq!(list.fetch_next_if_needed().unwrap().page, 2);
}

#[test]
fn github_search_fetches_more_only_when_the_filter_changes() {
    let mut ui = ui(Config {
        features: Features {
            enable_gh_notification: true,
        },
        ..Config::default()
    });
    assert!(matches!(
        ui.apply_terminal_event(&key!('/')),
        Operations::Nop
    ));
    assert!(matches!(
        ui.apply_terminal_event(&key!(backspace)),
        Operations::Nop
    ));
    assert!(matches!(
        ui.apply_terminal_event(&key!('x')),
        Operations::One(Operation::FetchGhNotifications { .. })
    ));
    assert!(matches!(
        ui.apply_terminal_event(&key!('y')),
        Operations::Nop
    ));
}

#[test]
fn github_filter_is_only_drawn_and_dismissed_on_its_tab() {
    use crate::test_support::screen::Screen;
    use ratatui::{Terminal, backend::TestBackend};

    let mut ui = ui(Config {
        features: Features {
            enable_gh_notification: true,
        },
        ..Config::default()
    });
    let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
    let mut popup_visible = |ui: &Compositor| {
        let frame = terminal
            .draw(|frame| ui.render(frame, DateTime::UNIX_EPOCH))
            .unwrap();
        Screen::new(frame.buffer).contains_text("─Filter─")
    };

    let _ = ui.apply_terminal_event(&key!('f'));
    assert!(popup_visible(&ui));
    let _ = ui.apply_terminal_event(&key!(tab));
    assert!(!popup_visible(&ui));
    let _ = ui.apply_terminal_event(&key!(esc));
    let _ = ui.apply_terminal_event(&key!(tab));
    let _ = ui.apply_terminal_event(&key!(tab));
    assert!(popup_visible(&ui));
    let _ = ui.apply_terminal_event(&key!(esc));
    assert!(!popup_visible(&ui));
}
