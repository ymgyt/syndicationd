#![cfg(feature = "integration")]

use std::time::Duration;

use serde_json::json;
use synd_client::payload;
use synd_term::{
    application::{
        Application, Cache, Config, FeedApiSession, FeedBackend, SessReady, StartSession,
        TermReady,
        outbound::feed::{MockFeedApi, MockFeedApiResponse},
    },
    config::Categories,
    integration::{
        event_stream, focus_gained_event, focus_lost_event, new_test_terminal, resize_event,
    },
    interact::mock::MockInteractor,
    test_support::screen::Screen,
    ui::theme::Theme,
};
use tempfile::TempDir;

mod established_session {
    use super::*;

    #[tokio::test]
    async fn initial_view() {
        let (_cache_dir, mut app) = start_app().await;

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
    async fn terminal_events() {
        let (_cache_dir, mut app) = start_app().await;
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

async fn start_app() -> (TempDir, Application<TermReady, SessReady>) {
    let payload = initial_feed_view();
    let api = MockFeedApi::new([MockFeedApiResponse::InitialFeedView(Ok(payload))]);
    let (cache_dir, app) = app(api);

    let app = app.assume_terminal_ready();
    let app = match app.start_session().await {
        StartSession::Ready(app) => app,
        StartSession::Pending(_) => panic!("feed API session should already be established"),
    };

    (cache_dir, app)
}

fn app(feed_api: MockFeedApi) -> (TempDir, Application) {
    let cache_dir = tempfile::tempdir().expect("temp cache dir");
    let terminal = new_test_terminal(120, 30);
    let feed_backend = FeedBackend::with_api(feed_api, FeedApiSession::Established);
    let app = Application::builder()
        .terminal(terminal)
        .feed_backend(feed_backend)
        .categories(Categories::default_toml())
        .config(Config::default().with_idle_timer_interval(Duration::from_millis(10)))
        .cache(Cache::new(cache_dir.path().to_path_buf()))
        .theme(Theme::default())
        .interactor(Box::new(MockInteractor::new()))
        .build();

    (cache_dir, app)
}

fn initial_feed_view() -> payload::InitialFeedViewPayload {
    serde_json::from_value(json!({
        "subscriptions": {
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
                "refreshStatus": null,
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
        },
        "timeline": {
            "entries": {
                "nodes": [
                    {
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
                    },
                    {
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
                ],
                "pageInfo": {
                    "hasNextPage": false,
                    "endCursor": null
                }
            }
        }
    }))
    .expect("initial feed view fixture")
}
