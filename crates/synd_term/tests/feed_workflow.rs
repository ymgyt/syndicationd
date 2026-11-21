#![cfg(feature = "integration")]

use serde_json::json;
use synd_client::payload;
use synd_term::{
    application::{
        Application, Cache, Config,
        outbound::feed::{MockFeedApi, MockFeedApiResponse},
    },
    config::Categories,
    integration::{
        event_stream, focus_gained_event, focus_lost_event, new_test_terminal, resize_event,
    },
    interact::mock::MockInteractor,
    key,
    test_support::screen::Screen,
    ui::theme::Theme,
};
use tempfile::TempDir;

#[tokio::test]
async fn entering_feeds_requests_the_missing_subscription() {
    let api = MockFeedApi::new([MockFeedApiResponse::Subscription(Ok(subscription()))]);
    let (_cache_dir, mut app) = app(api);
    let (tx, mut input) = event_stream();
    tx.send(key!(tab));
    app.wait_until_jobs_completed(&mut input).await;
    assert!(Screen::new(app.buffer()).contains_text("Feed 1/1"));
}

mod bootstrapped_app {
    use super::*;

    #[tokio::test]
    async fn unsubscribe_popup_only_receives_keys_on_its_tab() {
        let (_cache_dir, mut app) = start_app();
        let (tx, mut input) = event_stream();
        app.wait_until_jobs_completed(&mut input).await;

        tx.send_multi([key!(tab), key!('d')]);
        app.wait_until_jobs_completed(&mut input).await;
        assert!(Screen::new(app.buffer()).contains_text("─Unsubscribe─"));

        tx.send_multi([key!(tab), key!('j')]);
        app.wait_until_jobs_completed(&mut input).await;
        assert!(Screen::new(app.buffer()).contains_text("Entry 2/2"));

        tx.send(key!(tab));
        app.wait_until_jobs_completed(&mut input).await;
        assert!(Screen::new(app.buffer()).contains_text("─Unsubscribe─"));
        tx.send(key!(esc));
        app.wait_until_jobs_completed(&mut input).await;
        assert!(!Screen::new(app.buffer()).contains_text("─Unsubscribe─"));
    }

    #[tokio::test]
    async fn status_shows_loaded_timeline_entries_before_completion() {
        let (_cache_dir, mut app) = start_app();
        let (_tx, mut input) = event_stream();
        let progress = format!("Fetch timeline 2/{}", Config::default().entries_limit);
        tokio::time::timeout(
            std::time::Duration::from_secs(5),
            app.run_until(&mut input, |buffer| {
                Screen::new(buffer).contains_text(&progress)
            }),
        )
        .await
        .unwrap()
        .unwrap();

        app.wait_until_jobs_completed(&mut input).await;
        assert!(!Screen::new(app.buffer()).contains_text(&progress));
    }

    #[tokio::test]
    async fn initial_view() {
        let (_cache_dir, mut app) = start_app();

        let (_tx, mut input) = event_stream();
        app.wait_until_jobs_completed(&mut input).await;

        let screen = Screen::new(app.buffer());
        assert!(screen.contains_text("Syndicationd"));
        assert!(screen.contains_text("Entry 1/2"));
        assert!(screen.contains_text("Rust feed architecture"));
        assert!(screen.contains_text("Async GraphQL testing"));
        assert!(screen.contains_text("Engineering Notes"));
    }

    #[tokio::test]
    async fn search_applies_to_both_feed_views() {
        let (_cache_dir, mut app) = start_app();
        let (tx, mut input) = event_stream();
        app.wait_until_jobs_completed(&mut input).await;

        tx.send_multi([key!(tab), key!('/'), key!('z'), key!(esc)]);
        app.wait_until_jobs_completed(&mut input).await;
        assert!(Screen::new(app.buffer()).contains_text("Feed -/-"));

        tx.send(key!(tab));
        app.wait_until_jobs_completed(&mut input).await;
        assert!(Screen::new(app.buffer()).contains_text("Entry -/-"));

        tx.send_multi([key!('/'), key!(backspace)]);
        app.wait_until_jobs_completed(&mut input).await;
        assert!(Screen::new(app.buffer()).contains_text("Entry 1/2"));
    }

    #[tokio::test]
    async fn category_hints_are_ready_when_the_filter_opens() {
        let (_cache_dir, mut app) = start_app();
        let (tx, mut input) = event_stream();
        app.wait_until_jobs_completed(&mut input).await;
        tx.send(key!('c'));
        app.wait_until_jobs_completed(&mut input).await;
        assert!(
            Screen::new(app.buffer())
                .lines()
                .last()
                .unwrap()
                .contains("a:Toggle rust category")
        );
        tx.send(key!('a'));
        app.wait_until_jobs_completed(&mut input).await;
        assert!(Screen::new(app.buffer()).contains_text("Entry -/-"));
    }

    #[tokio::test]
    async fn terminal_events() {
        let (_cache_dir, mut app) = start_app();
        let (tx, mut input) = event_stream();
        app.wait_until_jobs_completed(&mut input).await;

        tx.send(focus_gained_event());
        tx.send(focus_lost_event());
        app.wait_until_jobs_completed(&mut input).await;

        let (mut columns, mut rows) = (120, 30);
        loop {
            columns /= 2;
            rows /= 2;
            if columns == 0 && rows == 0 {
                break;
            }

            tx.send(resize_event(columns, rows));
            app.wait_until_jobs_completed(&mut input).await;
        }
    }
}

fn start_app() -> (TempDir, Application) {
    let api = MockFeedApi::new([
        MockFeedApiResponse::FeedEvents(Ok(Vec::new())),
        MockFeedApiResponse::Subscription(Ok(subscription())),
        MockFeedApiResponse::TimelineEntries(Ok(timeline_entries())),
        MockFeedApiResponse::TimelineChanges(Ok(payload::TimelineChangesPayload {
            changes: Vec::new(),
            seq: 2,
            has_more: false,
        })),
    ]);
    let (cache_dir, mut app) = app(api);

    app.bootstrap_for_test();

    (cache_dir, app)
}

fn app(feed_api: MockFeedApi) -> (TempDir, Application) {
    let cache_dir = tempfile::tempdir().expect("temp cache dir");
    let terminal = new_test_terminal(120, 30);
    let app = Application::builder()
        .terminal(terminal)
        .feed_api(feed_api)
        .categories(Categories::default_toml())
        .config(Config::default())
        .cache(Cache::new(cache_dir.path().to_path_buf()))
        .theme(Theme::default())
        .interactor(Box::new(MockInteractor::new()))
        .build();

    (cache_dir, app)
}

fn subscription() -> payload::SubscriptionPayload {
    serde_json::from_value(json!({
        "feeds": {
            "nodes": [{
                "url": "https://example.com/feed.xml",
                "requirement": "SHOULD",
                "category": "rust",
                "crawlPolicy": {
                    "polling": {
                        "kind": "INTERVAL",
                        "intervalSeconds": 3600
                    }
                },
                "feed": {
                    "type": "RSS2",
                    "title": "Engineering Notes",
                    "updated": null,
                    "websiteUrl": "https://example.com",
                    "description": null,
                    "generator": null,
                    "entries": {
                        "nodes": [],
                        "pageInfo": {
                            "hasNextPage": false,
                            "endCursor": null
                        }
                    },
                    "links": {
                        "nodes": []
                    },
                    "authors": {
                        "nodes": []
                    }
                }
            }],
            "pageInfo": {
                "hasNextPage": false,
                "endCursor": null
            }
        }
    }))
    .expect("subscription fixture")
}

fn timeline_entries() -> payload::TimelineEntryConnection {
    serde_json::from_value(json!({
        "nodes": [
            {
                "orderTime": "2026-06-02T00:00:00Z",
                "entry": {
                    "id": "synd:entry:v1:0000000000000000000000000000000000000000000000000000000000000001",
                    "title": "Rust feed architecture",
                    "published": null,
                    "updated": null,
                    "websiteUrl": "https://example.com/rust-feed-architecture",
                    "summary": "A note about feed architecture.",
                    "feed": {
                        "title": "Engineering Notes",
                        "url": "https://example.com/feed.xml",
                        "requirement": "SHOULD",
                        "category": "rust"
                    }
                }
            },
            {
                "orderTime": "2026-06-01T00:00:00Z",
                "entry": {
                    "id": "synd:entry:v1:0000000000000000000000000000000000000000000000000000000000000002",
                    "title": "Async GraphQL testing",
                    "published": null,
                    "updated": null,
                    "websiteUrl": "https://example.com/async-graphql-testing",
                    "summary": "A note about testing async GraphQL code.",
                    "feed": {
                        "title": "Engineering Notes",
                        "url": "https://example.com/feed.xml",
                        "requirement": "SHOULD",
                        "category": "rust"
                    }
                }
            }
        ],
        "pageInfo": {
            "hasNextPage": false,
            "endCursor": null
        },
        "seq": 2
    }))
    .expect("timeline entries fixture")
}
