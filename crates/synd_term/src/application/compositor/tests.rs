use super::*;
use crate::{
    application::{Features, Generation, Populate, RequestId, RequestKind},
    event::{Event, FeedRequestEvent, FeedsEvent, GhEvent},
    key,
    operation::Operation,
    types::gh::Notification,
};

fn new_compositor(config: Config) -> Compositor {
    Compositor::new(config, Theme::default(), Categories::default_toml(), false)
}

#[test]
fn failed_login_shows_the_error_and_allows_retry_with_the_selected_provider() {
    use crate::{
        application::RequestError, auth::AuthenticationProvider, test_support::screen::Screen,
    };
    use ratatui::{Terminal, backend::TestBackend};

    let mut compositor = Compositor::new(
        Config::default(),
        Theme::default(),
        Categories::default_toml(),
        true,
    );
    let _ = compositor.apply_terminal_event(&key!('j'));
    assert!(matches!(
        compositor.apply_terminal_event(&key!(enter)),
        Operations::One(Operation::StartDeviceFlow {
            provider: AuthenticationProvider::Google
        })
    ));
    assert!(matches!(
        compositor.apply_terminal_event(&key!(enter)),
        Operations::Nop
    ));

    let request_id = RequestId::new(1);
    let kind = RequestKind::StartDeviceFlow {
        provider: AuthenticationProvider::Google,
    };
    let _ = compositor.apply_event(&Event::RequestEmitted {
        request_id,
        kind: kind.clone(),
    });
    let _ = compositor.apply_event(&Event::RequestCompleted {
        request_id,
        kind,
        result: Err(RequestError::Authentication(anyhow::anyhow!(
            "authorization unavailable"
        ))),
    });

    let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
    let frame = terminal
        .draw(|frame| compositor.render(frame, DateTime::UNIX_EPOCH))
        .unwrap();
    assert!(Screen::new(frame.buffer).contains_text("authorization unavailable"));
    assert!(matches!(
        compositor.apply_terminal_event(&key!(enter)),
        Operations::One(Operation::StartDeviceFlow {
            provider: AuthenticationProvider::Google
        })
    ));
}

#[test]
fn replacing_a_credential_catches_up_the_existing_timeline() {
    let mut compositor = Compositor::new(
        Config::default(),
        Theme::default(),
        Categories::default_toml(),
        true,
    );
    let Operations::Many(initial) = compositor.apply_event(&Event::ApiCredentialConfigured) else {
        panic!("first credential should start feed synchronization");
    };
    assert!(matches!(
        initial.as_slice(),
        [
            Operation::WatchFeedEvents,
            Operation::FetchSubscription { .. },
            Operation::OpenInitialTimeline,
        ]
    ));
    let timeline_id = synd_client::payload::TimelineId::from_u128(42);
    let opened = Event::Feeds(FeedsEvent::Request {
        request_id: RequestId::new(1),
        event: FeedRequestEvent::InitialTimelineOpened { timeline_id },
    });
    assert!(matches!(
        compositor.apply_event(&opened),
        Operations::One(Operation::FetchTimelineWindow { timeline_id: id, .. }) if id == timeline_id
    ));
    let feeds = &mut compositor.components.feeds;
    feeds.apply_timeline_window_chunk(Vec::new(), 42);
    assert!(matches!(
        feeds.complete_timeline_window(true),
        Some(Operation::CatchUpTimeline { timeline_id: id, since: 42 }) if id == timeline_id
    ));
    assert!(feeds.complete_timeline_catch_up(true).is_none());

    assert!(matches!(
        compositor.apply_event(&Event::ApiCredentialConfigured),
        Operations::One(Operation::CatchUpTimeline { timeline_id: id, since: 42 }) if id == timeline_id
    ));
}

#[test]
fn entering_feeds_requests_the_configured_page_size() {
    let mut compositor = new_compositor(Config {
        feeds_per_pagination: 17,
        ..Config::default()
    });
    assert!(matches!(
        compositor.apply_terminal_event(&key!(tab)),
        Operations::One(Operation::FetchSubscription {
            populate: Populate::Replace,
            after: None,
            first: 17,
            ..
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
    let mut compositor = new_compositor(Config {
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
        assert!(
            matches!(compositor.resolve_keymap(key.into()), Some(Command::List {
            to, command: ListCommand::Last,
        }) if to == owner)
        );
        let _ = compositor.apply_terminal_event(&key!(tab));
    }
    let _ = compositor.apply_terminal_event(&key!('f'));
    assert!(compositor.resolve_keymap(key.into()).is_none());
    let _ = compositor.apply_terminal_event(&key!(esc));
    assert!(matches!(
        compositor.resolve_keymap(key.into()),
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

struct PageRequest {
    id: RequestId,
    kind: RequestKind,
    populate: Populate,
    generation: Generation,
}

fn start_page(compositor: &mut Compositor, id: RequestId, operation: Operations) -> PageRequest {
    let Operations::One(Operation::FetchGhNotifications {
        generation,
        populate,
        params,
    }) = operation
    else {
        panic!("expected one GitHub page request, got {operation:?}");
    };
    let kind = RequestKind::FetchGhNotifications {
        generation,
        page: params.page,
    };
    assert!(matches!(
        compositor.apply_event(&Event::RequestEmitted {
            request_id: id,
            kind: kind.clone()
        }),
        Operations::Nop
    ));
    PageRequest {
        id,
        kind,
        populate,
        generation,
    }
}

fn deliver_page(
    compositor: &mut Compositor,
    page: PageRequest,
    notification_id: u64,
) -> Operations {
    let operations = compositor.apply_event(&Event::Gh {
        request_id: page.id,
        event: GhEvent::NotificationsFetched {
            generation: page.generation,
            populate: page.populate,
            notifications: vec![notification(notification_id)],
        },
    });
    assert!(matches!(
        compositor.apply_event(&Event::RequestCompleted {
            request_id: page.id,
            kind: page.kind,
            result: Ok(())
        }),
        Operations::Nop
    ));
    operations
}

#[test]
fn previous_lists_next_page_arriving_after_reload_is_ignored() {
    let mut compositor = new_compositor(Config {
        features: Features {
            enable_gh_notification: true,
        },
        ..Config::default()
    });
    let first = RequestId::new(1);
    let old_page = RequestId::new(2);
    let refreshed = RequestId::new(3);

    let fetch = compositor.apply_terminal_event(&key!('r'));
    let first_page = start_page(&mut compositor, first, fetch);
    let fetch = deliver_page(&mut compositor, first_page, 1);
    let append = start_page(&mut compositor, old_page, fetch);
    assert_eq!(append.populate, Populate::Append);

    let fetch = compositor.apply_terminal_event(&key!('r'));
    let replace = start_page(&mut compositor, refreshed, fetch);
    assert_eq!(replace.populate, Populate::Replace);
    let next = deliver_page(&mut compositor, replace, 2);
    assert!(
        matches!(next, Operations::One(Operation::FetchGhNotifications { params, .. }) if params.page == 2)
    );

    assert!(matches!(
        deliver_page(&mut compositor, append, 3),
        Operations::Nop
    ));
    let list = &compositor.components.gh.notifications;
    assert_eq!(list.notification_ids(), vec![2u64.into()]);
    assert_eq!(list.fetch_next_if_needed().unwrap().page, 2);
}

#[test]
fn failed_page_from_previous_list_finishes_without_disturbing_the_current_fetch() {
    use crate::{application::RequestError, client::gh::GhError, test_support::screen::Screen};
    use ratatui::{Terminal, backend::TestBackend};

    let mut compositor = new_compositor(Config {
        features: Features {
            enable_gh_notification: true,
        },
        ..Config::default()
    });
    let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
    let mut status = |compositor: &Compositor| {
        let frame = terminal
            .draw(|frame| compositor.render(frame, DateTime::UNIX_EPOCH))
            .unwrap();
        Screen::new(frame.buffer).lines().pop().unwrap()
    };
    let failure = |page: PageRequest| Event::RequestCompleted {
        request_id: page.id,
        kind: page.kind,
        result: Err(RequestError::Gh(GhError::SecondaryRateLimit)),
    };

    let fetch = compositor.apply_terminal_event(&key!('r'));
    let old = start_page(&mut compositor, RequestId::new(1), fetch);
    let fetch = compositor.apply_terminal_event(&key!('r'));
    let current = start_page(&mut compositor, RequestId::new(2), fetch);
    assert!(status(&compositor).contains("(+1)"));

    let _ = compositor.apply_event(&failure(old));
    let remaining = status(&compositor);
    assert!(remaining.contains("Fetch GitHub notifications page 1"));
    assert!(!remaining.contains("(+1)"));
    let _ = compositor.apply_terminal_event(&key!('/'));
    assert!(matches!(
        compositor.apply_terminal_event(&key!('x')),
        Operations::Nop
    ));

    let _ = compositor.apply_event(&failure(current));
    assert!(!compositor.is_busy());
    assert!(status(&compositor).contains("secondary rate limits exceeded"));
    assert!(matches!(
        compositor.apply_terminal_event(&key!('y')),
        Operations::One(Operation::FetchGhNotifications { .. })
    ));
}

#[test]
fn github_search_fetches_more_only_when_the_filter_changes() {
    let mut compositor = new_compositor(Config {
        features: Features {
            enable_gh_notification: true,
        },
        ..Config::default()
    });
    assert!(matches!(
        compositor.apply_terminal_event(&key!('/')),
        Operations::Nop
    ));
    assert!(matches!(
        compositor.apply_terminal_event(&key!(backspace)),
        Operations::Nop
    ));
    assert!(matches!(
        compositor.apply_terminal_event(&key!('x')),
        Operations::One(Operation::FetchGhNotifications { .. })
    ));
    assert!(matches!(
        compositor.apply_terminal_event(&key!('y')),
        Operations::Nop
    ));
}

#[test]
fn github_filter_is_only_drawn_and_dismissed_on_its_tab() {
    use crate::test_support::screen::Screen;
    use ratatui::{Terminal, backend::TestBackend};

    let mut compositor = new_compositor(Config {
        features: Features {
            enable_gh_notification: true,
        },
        ..Config::default()
    });
    let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
    let mut popup_visible = |compositor: &Compositor| {
        let frame = terminal
            .draw(|frame| compositor.render(frame, DateTime::UNIX_EPOCH))
            .unwrap();
        Screen::new(frame.buffer).contains_text("─Filter─")
    };

    let _ = compositor.apply_terminal_event(&key!('f'));
    assert!(popup_visible(&compositor));
    let _ = compositor.apply_terminal_event(&key!(tab));
    assert!(!popup_visible(&compositor));
    let _ = compositor.apply_terminal_event(&key!(esc));
    let _ = compositor.apply_terminal_event(&key!(tab));
    let _ = compositor.apply_terminal_event(&key!(tab));
    assert!(popup_visible(&compositor));
    let _ = compositor.apply_terminal_event(&key!(esc));
    assert!(!popup_visible(&compositor));
}

fn deliver_subscription_page(
    compositor: &mut Compositor,
    id: u64,
    fetch: &Operations,
    url: &str,
    next_cursor: Option<&str>,
) -> Operations {
    use crate::event::{FeedRequestEvent, FeedsEvent};

    let Operations::One(Operation::FetchSubscription {
        generation,
        populate,
        ..
    }) = fetch
    else {
        panic!("expected a subscription page request, got {fetch:?}");
    };
    let request_id = RequestId::new(id);
    let subscription = serde_json::from_value(serde_json::json!({
        "feeds": {
            "nodes": [{ "url": url, "crawlPolicy": { "polling": { "kind": "MANUAL" } } }],
            "pageInfo": { "hasNextPage": next_cursor.is_some(), "endCursor": next_cursor }
        }
    }))
    .unwrap();
    let kind = RequestKind::FetchSubscription {
        generation: *generation,
    };
    let _ = compositor.apply_event(&Event::RequestEmitted {
        request_id,
        kind: kind.clone(),
    });
    let next = compositor.apply_event(&Event::Feeds(FeedsEvent::Request {
        request_id,
        event: FeedRequestEvent::SubscriptionFetched {
            generation: *generation,
            populate: *populate,
            subscription,
        },
    }));
    let _ = compositor.apply_event(&Event::RequestCompleted {
        request_id,
        kind,
        result: Ok(()),
    });
    next
}

#[test]
fn reloading_subscriptions_discards_old_pages_and_their_cursors() {
    for pending_append in [false, true] {
        let mut compositor = new_compositor(Config::default());
        let first = compositor.apply_terminal_event(&key!(tab));
        let old = if pending_append {
            deliver_subscription_page(
                &mut compositor,
                1,
                &first,
                "https://example.com/first",
                Some("next"),
            )
        } else {
            first
        };

        let refresh = compositor.apply_terminal_event(&key!('r'));
        let current = "https://example.com/current";
        let obsolete = "https://example.com/obsolete";
        let next =
            deliver_subscription_page(&mut compositor, 2, &refresh, current, Some("current-next"));
        assert!(
            matches!(&next, Operations::One(Operation::FetchSubscription {
            after: Some(cursor), populate: Populate::Append, ..
        }) if cursor == "current-next")
        );
        assert!(matches!(
            deliver_subscription_page(&mut compositor, 3, &old, obsolete, Some("obsolete-next")),
            Operations::Nop
        ));
        let last = "https://example.com/last";
        let _ = deliver_subscription_page(&mut compositor, 4, &next, last, None);
        let feeds = &compositor.components.feeds;
        assert!(feeds.is_already_subscribed(&current.try_into().unwrap()));
        assert!(feeds.is_already_subscribed(&last.try_into().unwrap()));
        assert!(!feeds.is_already_subscribed(&obsolete.try_into().unwrap()));
    }
}

#[test]
fn only_the_current_subscription_failure_reaches_the_status_line() {
    use crate::{application::RequestError, test_support::screen::Screen};
    use ratatui::{Terminal, backend::TestBackend};

    let mut compositor = new_compositor(Config::default());
    let mut start = |id, key| {
        let Operations::One(Operation::FetchSubscription { generation, .. }) =
            compositor.apply_terminal_event(&key)
        else {
            panic!("expected a subscription fetch")
        };
        let request_id = RequestId::new(id);
        let kind = RequestKind::FetchSubscription { generation };
        let _ = compositor.apply_event(&Event::RequestEmitted {
            request_id,
            kind: kind.clone(),
        });
        Event::RequestCompleted {
            request_id,
            kind,
            result: Err(RequestError::SyndApi(
                synd_client::SyndApiError::UnexpectedResponse {
                    context: "page failed",
                },
            )),
        }
    };
    let old = start(1, key!(tab));
    let current = start(2, key!('r'));
    let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
    let mut status = |compositor: &Compositor| {
        let frame = terminal
            .draw(|frame| compositor.render(frame, DateTime::UNIX_EPOCH))
            .unwrap();
        Screen::new(frame.buffer).lines().pop().unwrap()
    };
    let _ = compositor.apply_event(&old);
    assert!(compositor.is_busy());
    assert!(!status(&compositor).contains("page failed"));
    let _ = compositor.apply_event(&current);
    assert!(!compositor.is_busy());
    assert!(status(&compositor).contains("page failed"));
}

#[test]
fn unsubscribe_uses_the_feed_shown_when_confirmation_opened() {
    let mut compositor = new_compositor(Config::default());
    let first = compositor.apply_terminal_event(&key!(tab));
    let _ = deliver_subscription_page(
        &mut compositor,
        1,
        &first,
        "https://example.com/first",
        None,
    );
    let _ = compositor.apply_terminal_event(&key!('d'));
    let refresh = compositor.components.feeds.reload_subscription().into();
    let _ = deliver_subscription_page(
        &mut compositor,
        2,
        &refresh,
        "https://example.com/second",
        None,
    );
    assert!(matches!(compositor.apply_terminal_event(&key!(enter)),
        Operations::One(Operation::UnsubscribeFeed { url }) if url.as_str() == "https://example.com/first"
    ));

    let _ = compositor.apply_terminal_event(&key!('d'));
    assert!(matches!(
        compositor.apply_terminal_event(&key!(esc)),
        Operations::Nop
    ));
    let _ = compositor.apply_terminal_event(&key!('d'));
    let _ = compositor.apply_terminal_event(&key!('l'));
    assert!(matches!(
        compositor.apply_terminal_event(&key!(enter)),
        Operations::Nop
    ));
}

#[test]
fn status_line_shows_the_main_actions_for_each_list() {
    use crate::test_support::screen::Screen;
    use ratatui::{Terminal, backend::TestBackend};

    let mut compositor = new_compositor(Config {
        features: Features {
            enable_gh_notification: true,
        },
        ..Config::default()
    });
    let mut terminal = Terminal::new(TestBackend::new(80, 20)).unwrap();
    let mut status = |compositor: &Compositor| {
        let frame = terminal
            .draw(|frame| compositor.render(frame, DateTime::UNIX_EPOCH))
            .unwrap();
        Screen::new(frame.buffer)
            .lines()
            .pop()
            .unwrap()
            .trim()
            .to_owned()
    };
    assert_eq!(
        status(&compositor),
        "Tab:󰹳  j/k:󰹹  Ent:󰏌  d:  f:󰈶  /:  q:"
    );
    let _ = compositor.apply_terminal_event(&key!('f'));
    assert!(status(&compositor).starts_with("esc:Apply filters"));
    let _ = compositor.apply_terminal_event(&key!(esc));
    let _ = compositor.apply_terminal_event(&key!(tab));
    let entries = status(&compositor);
    assert_eq!(entries, "Tab:󰹳  j/k:󰹹  Ent:󰏌  Sp:󰏋  /:  q:");
    let _ = compositor.apply_terminal_event(&key!('g'));
    assert_eq!(status(&compositor), "g:First  e:Last");
    let _ = compositor.apply_terminal_event(&key!('e'));
    assert_eq!(status(&compositor), entries);
    let _ = compositor.apply_terminal_event(&key!(tab));
    assert_eq!(
        status(&compositor),
        "Tab:󰹳  j/k:󰹹  Ent:󰏌  a:󰑫  e:  d:󰼡  /:  q:"
    );
}

#[test]
fn status_hints_follow_remaps_and_search_input() {
    use crate::{keymap::CompiledKeymaps, test_support::screen::Screen};
    use ratatui::{Terminal, backend::TestBackend};
    let bindings = toml::from_str(
        r#"
        [entries]
        keymap = [
            { on = "enter", command = "nop" },
            { on = "space", command = "entries.open" },
            { on = ["z", "b"], command = "entries.browse" }
        ]
        [list]
        keymap = [
            { on = "j", command = "nop" },
            { on = "n", command = "list.next" }
        ]
        [filter]
        keymap = [
            { on = "/", command = "nop" },
            { on = "x", command = "filter.search", desc = "Find stories" }
        ]
    "#,
    )
    .unwrap();
    let mut compositor = new_compositor(Config {
        keymaps: CompiledKeymaps::default_with_user_config(bindings).unwrap(),
        ..Config::default()
    });
    let mut terminal = Terminal::new(TestBackend::new(240, 20)).unwrap();
    let mut status = |compositor: &Compositor| {
        let frame = terminal
            .draw(|frame| compositor.render(frame, DateTime::UNIX_EPOCH))
            .unwrap();
        Screen::new(frame.buffer).lines().pop().unwrap()
    };

    let normal = status(&compositor);
    assert!(normal.contains("n/k:󰹹"));
    assert!(normal.contains("Sp:󰏌  z b:󰏋"));
    assert!(!normal.contains("Ent:"));
    assert!(normal.contains("x:Find stories"));
    assert!(!normal.contains("/:Search"));
    let _ = compositor.apply_terminal_event(&key!('x'));
    let editing = status(&compositor);
    assert_eq!(
        editing.trim(),
        "esc:Close filter  backspace:Delete previous character"
    );
    let _ = compositor.apply_terminal_event(&key!(esc));
    assert_eq!(status(&compositor), normal);
}
