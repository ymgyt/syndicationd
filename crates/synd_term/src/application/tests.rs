use super::component::Screens;
use crate::{
    application::{Clock, Config},
    config::Categories,
    ui::theme::Theme,
};
use chrono::{DateTime, Utc};
use std::sync::Arc;

type Compositor = tuiframework::testing::TestScreen<Screens>;
fn make(config: Config, theme: Theme, categories: Categories, auth: bool) -> Compositor {
    struct Epoch;
    impl Clock for Epoch {
        fn now(&self) -> DateTime<Utc> {
            DateTime::UNIX_EPOCH
        }
    }
    let ui = Screens::new(&config, theme, categories, auth, Arc::new(Epoch));
    tuiframework::testing::TestScreen::new(ui, super::overlay::layout(), config.keymaps.0)
}

use crate::{
    application::{Features, Generation, Populate, RequestId, RequestKind},
    event::{Event, FeedRequestEvent, FeedsEvent, GhEvent},
    key,
    operation::Operation,
    types::gh::Notification,
};

fn new_compositor(config: Config) -> Compositor {
    make(config, Theme::default(), Categories::default_toml(), false)
}

#[test]
fn rotating_the_palette_preserves_the_transparent_background() {
    use crate::ui::theme::{Palette, ThemeBackground};
    use ratatui::{Terminal, backend::TestBackend, style::Color};

    let mut compositor = make(
        Config::default(),
        Theme::new(Palette::ferra()).with_background(ThemeBackground::Transparent),
        Categories::default_toml(),
        false,
    );
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    let mut surface_colors = |compositor: &Compositor| {
        let frame = terminal.draw(|frame| compositor.render(frame)).unwrap();
        let cell = &frame.buffer[(0, 10)];
        (cell.fg, cell.bg)
    };

    assert_eq!(
        surface_colors(&compositor),
        (Palette::ferra().foreground, Color::Reset)
    );
    let _ = compositor.apply_terminal_event(&crate::shift!('t'));
    assert_eq!(
        surface_colors(&compositor),
        (Palette::solarized_dark().foreground, Color::Reset)
    );
}

#[test]
fn failed_login_shows_the_error_and_allows_retry_with_the_selected_provider() {
    use crate::{
        application::RequestError, auth::AuthenticationProvider, test_support::screen::Screen,
    };
    use ratatui::{Terminal, backend::TestBackend};

    let mut compositor = make(
        Config::default(),
        Theme::default(),
        Categories::default_toml(),
        true,
    );
    let _ = compositor.apply_terminal_event(&key!('j'));
    assert!(matches!(
        compositor
            .apply_terminal_event(&key!(enter))
            .operations
            .as_slice(),
        [Operation::StartDeviceFlow {
            provider: AuthenticationProvider::Google
        }]
    ));
    assert!(matches!(
        compositor
            .apply_terminal_event(&key!(enter))
            .operations
            .as_slice(),
        []
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
    let frame = terminal.draw(|frame| compositor.render(frame)).unwrap();
    assert!(Screen::new(frame.buffer).contains_text("authorization unavailable"));
    assert!(matches!(
        compositor
            .apply_terminal_event(&key!(enter))
            .operations
            .as_slice(),
        [Operation::StartDeviceFlow {
            provider: AuthenticationProvider::Google
        }]
    ));
}

#[test]
fn replacing_a_credential_catches_up_the_existing_timeline() {
    let mut compositor = make(
        Config::default(),
        Theme::default(),
        Categories::default_toml(),
        true,
    );
    let initial = compositor.apply_event(&Event::ApiCredentialConfigured);
    assert!(matches!(
        initial.operations.as_slice(),
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
        compositor.apply_event(&opened).operations.as_slice(),
        [Operation::FetchTimelineWindow { timeline_id: id, .. }] if *id == timeline_id
    ));
    let feeds = &mut compositor.components.feeds;
    feeds.apply_timeline_window_chunk(Vec::new(), 42);
    assert!(matches!(
        feeds.complete_timeline_window(true),
        Some(Operation::CatchUpTimeline { timeline_id: id, since: 42 }) if id == timeline_id
    ));
    assert!(feeds.complete_timeline_catch_up(true).is_none());

    assert!(matches!(
        compositor.apply_event(&Event::ApiCredentialConfigured).operations.as_slice(),
        [Operation::CatchUpTimeline { timeline_id: id, since: 42 }] if *id == timeline_id
    ));
}

#[test]
fn entering_feeds_requests_the_configured_page_size() {
    let mut compositor = new_compositor(Config {
        feeds_per_pagination: 17,
        ..Config::default()
    });
    assert!(matches!(
        compositor
            .apply_terminal_event(&key!(tab))
            .operations
            .as_slice(),
        [Operation::FetchSubscription {
            populate: Populate::Replace,
            after: None,
            first: 17,
            ..
        }]
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
            matches!(compositor.resolve_key(key), Some((to, Command::List(ListCommand::Last))) if to == owner)
        );
        let _ = compositor.apply_terminal_event(&key!(tab));
    }
    let _ = compositor.apply_terminal_event(&key!('f'));
    assert!(compositor.resolve_key(key).is_none());
    let _ = compositor.apply_terminal_event(&key!(esc));
    assert!(matches!(
        compositor.resolve_key(key),
        Some((
            ComponentId::GhNotifications,
            Command::List(ListCommand::Last)
        ))
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

fn start_page(compositor: &mut Compositor, id: RequestId, operation: &[Operation]) -> PageRequest {
    let [
        Operation::FetchGhNotifications {
            generation,
            populate,
            params,
        },
    ] = operation
    else {
        panic!("expected one GitHub page request, got {operation:?}");
    };
    let kind = RequestKind::FetchGhNotifications {
        generation: *generation,
        page: params.page,
    };
    assert!(matches!(
        compositor
            .apply_event(&Event::RequestEmitted {
                request_id: id,
                kind: kind.clone()
            })
            .operations
            .as_slice(),
        []
    ));
    PageRequest {
        id,
        kind,
        populate: *populate,
        generation: *generation,
    }
}

fn deliver_page(
    compositor: &mut Compositor,
    page: PageRequest,
    notification_id: u64,
) -> Vec<Operation> {
    let operations = compositor
        .apply_event(&Event::Gh {
            request_id: page.id,
            event: GhEvent::NotificationsFetched {
                generation: page.generation,
                populate: page.populate,
                notifications: vec![notification(notification_id)],
            },
        })
        .operations;
    assert!(matches!(
        compositor
            .apply_event(&Event::RequestCompleted {
                request_id: page.id,
                kind: page.kind,
                result: Ok(())
            })
            .operations
            .as_slice(),
        []
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

    let fetch = compositor.apply_terminal_event(&key!('r')).operations;
    let first_page = start_page(&mut compositor, first, &fetch);
    let fetch = deliver_page(&mut compositor, first_page, 1);
    let append = start_page(&mut compositor, old_page, &fetch);
    assert_eq!(append.populate, Populate::Append);

    let fetch = compositor.apply_terminal_event(&key!('r')).operations;
    let replace = start_page(&mut compositor, refreshed, &fetch);
    assert_eq!(replace.populate, Populate::Replace);
    let next = deliver_page(&mut compositor, replace, 2);
    assert!(
        matches!(next.as_slice(), [Operation::FetchGhNotifications { params, .. }] if params.page == 2)
    );

    assert!(matches!(
        deliver_page(&mut compositor, append, 3).as_slice(),
        []
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
        let frame = terminal.draw(|frame| compositor.render(frame)).unwrap();
        Screen::new(frame.buffer).lines().pop().unwrap()
    };
    let failure = |page: PageRequest| Event::RequestCompleted {
        request_id: page.id,
        kind: page.kind,
        result: Err(RequestError::Gh(GhError::SecondaryRateLimit)),
    };

    let fetch = compositor.apply_terminal_event(&key!('r')).operations;
    let old = start_page(&mut compositor, RequestId::new(1), &fetch);
    let fetch = compositor.apply_terminal_event(&key!('r')).operations;
    let current = start_page(&mut compositor, RequestId::new(2), &fetch);
    assert!(status(&compositor).contains("(+1)"));

    let _ = compositor.apply_event(&failure(old));
    let remaining = status(&compositor);
    assert!(remaining.contains("Fetch GitHub notifications page 1"));
    assert!(!remaining.contains("(+1)"));
    let _ = compositor.apply_terminal_event(&key!('/'));
    assert!(matches!(
        compositor
            .apply_terminal_event(&key!('x'))
            .operations
            .as_slice(),
        []
    ));

    let _ = compositor.apply_event(&failure(current));
    assert!(!compositor.is_busy());
    assert!(status(&compositor).contains("secondary rate limits exceeded"));
    assert!(matches!(
        compositor
            .apply_terminal_event(&key!('y'))
            .operations
            .as_slice(),
        [Operation::FetchGhNotifications { .. }]
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
        compositor
            .apply_terminal_event(&key!('/'))
            .operations
            .as_slice(),
        []
    ));
    assert!(matches!(
        compositor
            .apply_terminal_event(&key!(backspace))
            .operations
            .as_slice(),
        []
    ));
    assert!(matches!(
        compositor
            .apply_terminal_event(&key!('x'))
            .operations
            .as_slice(),
        [Operation::FetchGhNotifications { .. }]
    ));
    assert!(matches!(
        compositor
            .apply_terminal_event(&key!('y'))
            .operations
            .as_slice(),
        []
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
        let frame = terminal.draw(|frame| compositor.render(frame)).unwrap();
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
    fetch: &[Operation],
    url: &str,
    next_cursor: Option<&str>,
) -> Vec<Operation> {
    use crate::event::{FeedRequestEvent, FeedsEvent};

    let [
        Operation::FetchSubscription {
            generation,
            populate,
            ..
        },
    ] = fetch
    else {
        panic!("expected a subscription page request, got {fetch:?}");
    };
    let request_id = RequestId::new(id);
    let subscription = serde_json::from_value(serde_json::json!({
        "feeds": {
            "nodes": [{ "url": url, "labels": {"category": "rust", "issue": "3"}, "crawlPolicy": { "polling": { "kind": "MANUAL" } } }],
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
    let next = compositor
        .apply_event(&Event::Feeds(FeedsEvent::Request {
            request_id,
            event: FeedRequestEvent::SubscriptionFetched {
                generation: *generation,
                populate: *populate,
                subscription,
            },
        }))
        .operations;
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
        let first = compositor.apply_terminal_event(&key!(tab)).operations;
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

        let refresh = compositor.apply_terminal_event(&key!('r')).operations;
        let current = "https://example.com/current";
        let obsolete = "https://example.com/obsolete";
        let next =
            deliver_subscription_page(&mut compositor, 2, &refresh, current, Some("current-next"));
        assert!(matches!(next.as_slice(), [Operation::FetchSubscription {
            after: Some(cursor), populate: Populate::Append, ..
        }] if cursor == "current-next"));
        assert!(matches!(
            deliver_subscription_page(&mut compositor, 3, &old, obsolete, Some("obsolete-next"))
                .as_slice(),
            []
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
        let operations = compositor.apply_terminal_event(&key).operations;
        let [Operation::FetchSubscription { generation, .. }] = operations.as_slice() else {
            panic!("expected a subscription fetch")
        };
        let request_id = RequestId::new(id);
        let kind = RequestKind::FetchSubscription {
            generation: *generation,
        };
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
        let frame = terminal.draw(|frame| compositor.render(frame)).unwrap();
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
fn subscribe_keeps_the_default_interval_as_a_placeholder() {
    use crate::{
        crossterm::event::{Event, Event::Paste, KeyCode, KeyEvent, KeyModifiers},
        keymap::CompiledKeymaps,
        test_support::screen::Screen,
    };
    use ratatui::{Terminal, backend::TestBackend};

    let bindings = toml::from_str(
        r#"
        [subscribe]
        keymap = [
            { on = "tab", command = "nop" },
            { on = "f2", command = "subscribe.next", desc = "Next field" }
        ]
    "#,
    )
    .unwrap();
    let mut compositor = new_compositor(Config {
        keymaps: CompiledKeymaps::default_with_user_config(bindings).unwrap(),
        ..Config::default()
    });
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    let mut screen = |compositor: &Compositor| {
        let frame = terminal.draw(|frame| compositor.render(frame)).unwrap();
        Screen::new(frame.buffer).lines()
    };
    for event in [
        key!(tab),
        key!('a'),
        Paste("https://example.org/feed.xml".into()),
    ] {
        let _ = compositor.apply_terminal_event(&event);
    }
    let lines = screen(&compositor);
    let status = lines.last().unwrap();
    assert!(status.contains("f2:Next field") && !status.contains("Tab:Next"));
    assert!(status.ends_with("esc:Cancel"));
    assert!(lines.iter().any(|line| line.contains("Poll every    24h")));

    let _ = compositor.apply_terminal_event(&Event::Key(KeyCode::F(2).into()));
    assert!(
        screen(&compositor)
            .iter()
            .any(|line| line.contains("Poll every    24h"))
    );
    let _ = compositor.apply_terminal_event(&Paste("30m".into()));
    assert!(
        screen(&compositor)
            .iter()
            .any(|line| line.contains("Poll every    30m"))
    );
    for _ in 0..3 {
        let _ = compositor.apply_terminal_event(&key!(backspace));
    }
    assert!(
        screen(&compositor)
            .iter()
            .any(|line| line.contains("Poll every    24h"))
    );
    let previous = Event::Key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT));
    let _ = compositor.apply_terminal_event(&previous);
    let submit = compositor.apply_terminal_event(&key!(enter));
    assert!(matches!(submit.operations.as_slice(),
        [Operation::SubscribeFeed { input, .. }] if input.crawl_policy.is_none()));
}

#[test]
fn failed_subscribe_keeps_the_draft_for_one_retry() {
    use crate::{
        application::RequestError, crossterm::event::Event::Paste, test_support::screen::Screen,
    };
    use ratatui::{Terminal, backend::TestBackend};

    let mut compositor = new_compositor(Config::default());
    for event in [
        key!(tab),
        key!('a'),
        Paste("https://example.org/feed".into()),
        key!(tab),
        Paste("30m".into()),
        key!(tab),
        Paste("k8s".into()),
        key!(tab),
        key!(tab),
        key!(tab),
    ] {
        let _ = compositor.apply_terminal_event(&event);
    }
    let submit = compositor.apply_terminal_event(&key!(enter));
    let [Operation::SubscribeFeed { input }] = submit.operations.as_slice() else {
        panic!("expected a subscription");
    };
    assert_eq!(input.labels.category().unwrap().as_str(), "kubernetes");

    let finish = |compositor: &mut Compositor, id, url, result| {
        let request_id = RequestId::new(id);
        let kind = RequestKind::SubscribeFeed { url };
        let _ = compositor.apply_event(&Event::RequestEmitted {
            request_id,
            kind: kind.clone(),
        });
        let _ = compositor.apply_event(&Event::RequestCompleted {
            request_id,
            kind,
            result,
        });
    };
    finish(
        &mut compositor,
        1,
        "https://example.org/other".try_into().unwrap(),
        Ok(()),
    );
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    let frame = terminal.draw(|frame| compositor.render(frame)).unwrap();
    assert!(Screen::new(frame.buffer).contains_text("Subscribing…"));
    for event in [Paste("/extra".into()), key!(enter)] {
        assert!(
            compositor
                .apply_terminal_event(&event)
                .operations
                .is_empty()
        );
    }

    finish(
        &mut compositor,
        2,
        input.url.clone(),
        Err(RequestError::SyndApi(
            synd_client::SyndApiError::UnexpectedResponse {
                context: "try again",
            },
        )),
    );
    let frame = terminal.draw(|frame| compositor.render(frame)).unwrap();
    let lines = Screen::new(frame.buffer).lines();
    assert!(
        lines
            .iter()
            .any(|line| line.contains("Could not subscribe:"))
    );
    assert!(lines.iter().any(|line| line.contains("try again")));
    assert!(!lines.last().unwrap().contains("try again"));
    assert!(lines.iter().any(|line| line.contains("k8s")));

    let retry = compositor.apply_terminal_event(&key!(enter));
    assert!(matches!(retry.operations.as_slice(),
        [Operation::SubscribeFeed { input: retried }] if retried == input
    ));
    finish(&mut compositor, 3, input.url.clone(), Ok(()));
    let frame = terminal.draw(|frame| compositor.render(frame)).unwrap();
    assert!(Screen::new(frame.buffer).contains_text("a:Subscribe"));
    assert!(!Screen::new(frame.buffer).contains_text("Subscribe to feed"));
}

#[test]
fn narrow_subscribe_keeps_the_invalid_label_and_its_error_together() {
    use crate::{crossterm::event::Event::Paste, test_support::screen::Screen};
    use ratatui::{Terminal, backend::TestBackend};

    let mut compositor = new_compositor(Config::default());
    let url = "https://example.org/long/path/to/a/feed/with/a/long/name.xml";
    for event in [
        key!(tab),
        key!('a'),
        Paste(url.into()),
        key!(tab),
        key!(tab),
        key!(tab),
        key!(enter), // Remove the initial category.
        key!(enter),
        Paste("priority".into()),
        key!(enter),
        Paste("{".into()),
        key!(tab),
        key!(enter), // Confirm the value with Done.
    ] {
        let _ = compositor.apply_terminal_event(&event);
    }
    let mut terminal = Terminal::new(TestBackend::new(44, 14)).unwrap();
    let frame = terminal.draw(|frame| compositor.render(frame)).unwrap();
    let lines = Screen::new(frame.buffer).lines();
    assert!(lines[0].contains("Feeds"));
    let error = lines
        .iter()
        .position(|line| line.contains("Enter a JSON value"))
        .unwrap();
    assert!(lines[error - 1].contains('{'));
    assert!(lines.iter().any(|line| line.contains("Cancel")));

    for event in [
        Paste("}".into()),
        key!(enter),
        key!(tab),
        key!(tab),
        key!(tab),
    ] {
        let _ = compositor.apply_terminal_event(&event);
    }
    let submit = compositor.apply_terminal_event(&key!(enter));
    let [Operation::SubscribeFeed { input, .. }] = submit.operations.as_slice() else {
        panic!("expected the corrected label to be accepted");
    };
    assert_eq!(input.url.as_str(), url);
    assert_eq!(
        serde_json::to_value(&input.labels).unwrap(),
        serde_json::json!({ "priority": {} })
    );
}

#[test]
fn edit_removes_category_and_preserves_other_labels_and_manual_polling() {
    use crate::test_support::screen::Screen;
    use ratatui::{Terminal, backend::TestBackend};
    use synd_client::payload::{CrawlPolicyInput, PollingPolicyInput};

    let mut compositor = new_compositor(Config::default());
    let fetch = compositor.apply_terminal_event(&key!(tab)).operations;
    let url = "https://example.com/feed";
    let _ = deliver_subscription_page(&mut compositor, 1, &fetch, url, None);
    let _ = compositor.apply_terminal_event(&key!('e'));
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    let frame = terminal.draw(|frame| compositor.render(frame)).unwrap();
    let screen = Screen::new(frame.buffer);
    for text in ["Edit subscription", url, "manual", "rust", "[ Save ]"] {
        assert!(screen.contains_text(text), "{text}");
    }

    for key in [key!(tab), key!(enter), key!(tab), key!(tab), key!(tab)] {
        let _ = compositor.apply_terminal_event(&key);
    }
    let save = compositor.apply_terminal_event(&key!(enter));
    let [Operation::SubscribeFeed { input }] = save.operations.as_slice() else {
        panic!(
            "expected Save to edit the selected subscription, got {:?}",
            save.operations
        );
    };
    assert_eq!(input.url.as_str(), url);
    assert_eq!(
        serde_json::to_value(&input.labels).unwrap(),
        serde_json::json!({"issue": "3"})
    );
    assert_eq!(
        input.crawl_policy,
        Some(CrawlPolicyInput {
            polling: PollingPolicyInput::Manual
        })
    );
}

#[test]
fn unsubscribe_uses_the_feed_shown_when_confirmation_opened() {
    let mut compositor = new_compositor(Config::default());
    let first = compositor.apply_terminal_event(&key!(tab)).operations;
    let _ = deliver_subscription_page(
        &mut compositor,
        1,
        &first,
        "https://example.com/first",
        None,
    );
    let _ = compositor.apply_terminal_event(&key!('d'));
    let refresh = [compositor.components.feeds.reload_subscription()];
    let _ = deliver_subscription_page(
        &mut compositor,
        2,
        &refresh,
        "https://example.com/second",
        None,
    );
    let _ = compositor.apply_terminal_event(&key!(tab));
    let submit = compositor.apply_terminal_event(&key!(enter));
    let [Operation::UnsubscribeFeed { url }] = submit.operations.as_slice() else {
        panic!("expected unsubscribe");
    };
    assert_eq!(url.as_str(), "https://example.com/first");
    assert!(
        compositor
            .apply_terminal_event(&key!(enter))
            .operations
            .is_empty()
    );

    let kind = RequestKind::UnsubscribeFeed { url: url.clone() };
    let finish = |compositor: &mut Compositor, id, result| {
        let request_id = RequestId::new(id);
        let _ = compositor.apply_event(&Event::RequestEmitted {
            request_id,
            kind: kind.clone(),
        });
        let _ = compositor.apply_event(&Event::RequestCompleted {
            request_id,
            kind: kind.clone(),
            result,
        });
    };
    finish(
        &mut compositor,
        3,
        Err(crate::application::RequestError::SyndApi(
            synd_client::SyndApiError::UnexpectedResponse {
                context: "try again",
            },
        )),
    );
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(44, 14)).unwrap();
    let frame = terminal.draw(|frame| compositor.render(frame)).unwrap();
    assert!(crate::test_support::screen::Screen::new(frame.buffer).contains_text("try again"));
    let retry = compositor.apply_terminal_event(&key!(enter));
    assert!(
        matches!(retry.operations.as_slice(), [Operation::UnsubscribeFeed { url: retried }] if retried == url)
    );
    finish(&mut compositor, 4, Ok(()));
    let frame = terminal.draw(|frame| compositor.render(frame)).unwrap();
    assert!(
        !crate::test_support::screen::Screen::new(frame.buffer).contains_text("[ Unsubscribe ]")
    );

    let _ = compositor.apply_terminal_event(&key!('d'));
    assert!(
        compositor
            .apply_terminal_event(&key!(enter))
            .operations
            .is_empty()
    );
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
        let frame = terminal.draw(|frame| compositor.render(frame)).unwrap();
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
        "Tab:󰹳  j/k:󰹹  Ent:󰏌  a:Subscribe  e:  d:󰼡  /:  q:"
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
        let frame = terminal.draw(|frame| compositor.render(frame)).unwrap();
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
