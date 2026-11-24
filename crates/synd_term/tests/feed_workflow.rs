#![cfg(feature = "integration")]

use std::sync::Arc;

use serde_json::json;
use synd_client::payload;
use synd_support::html::HtmlConverter;
use synd_term::{
    application::{
        Application, Cache, Config, Features,
        outbound::feed::{MockFeedApi, MockFeedApiResponse},
        outbound::gh::GhClient,
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
    let (_cache_dir, mut app) = app(api, None, None);
    let (tx, mut input) = event_stream();
    tx.send(key!(tab));
    app.wait_until_jobs_completed(&mut input).await;
    assert!(Screen::new(app.buffer()).contains_text("Feed 1/1"));
}

#[tokio::test]
async fn space_reads_the_selected_article_url_and_escape_returns_to_the_entry() {
    use axum::{Router, response::Html, routing::get};
    use std::fmt::Write as _;
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let article_url = format!("http://{}/article", listener.local_addr().unwrap());
    let publisher = Router::new().route(
        "/article",
        get(|| async {
            let mut paragraphs = String::new();
            for number in 1..=45 {
                write!(
                    paragraphs,
                    "<p>Reading this paragraph about topic {number}.</p>"
                )
                .unwrap();
            }
            Html(format!(
                "<html><head><title>Publisher title</title></head><body>
            <nav>Site navigation</nav><article><h2>First section</h2>{paragraphs}</article>
            </body></html>"
            ))
        }),
    );
    let server = tokio::spawn(async move { axum::serve(listener, publisher).await.unwrap() });
    let mut entries = timeline_entries();
    entries.nodes[1].entry.website_url = Some(article_url);
    let (_cache_dir, mut app) = start_app_with_entries(entries, None);
    let (tx, mut input) = event_stream();
    app.wait_until_jobs_completed(&mut input).await;

    tx.send_multi([key!('j'), key!(' ')]);
    app.wait_until_jobs_completed(&mut input).await;
    let screen = Screen::new(app.buffer());
    let initial = screen.lines();
    assert_eq!(initial[0].trim(), "Publisher title");
    assert!(screen.contains_text("First section"));
    assert!(!screen.contains_text("Site navigation"));

    let body = &initial[1..initial.len() - 1];
    let half = body.len() / 2;
    tx.send_multi([key!('z'), key!('j')]);
    app.wait_until_jobs_completed(&mut input).await;
    let scrolled = Screen::new(app.buffer()).lines();
    assert_eq!(scrolled[0], initial[0]);
    assert_eq!(scrolled[1..1 + body.len() - half], body[half..]);

    tx.send_multi([key!('z'), key!('k')]);
    app.wait_until_jobs_completed(&mut input).await;
    let restored = Screen::new(app.buffer()).lines();
    assert_eq!(restored[1..restored.len() - 1], *body);

    tx.send(key!(' '));
    app.wait_until_jobs_completed(&mut input).await;
    let screen = Screen::new(app.buffer());
    assert!(screen.contains_text("Reading this paragraph"));
    assert!(!screen.contains_text("First section"));

    tx.send(key!(esc));
    app.wait_until_jobs_completed(&mut input).await;
    let screen = Screen::new(app.buffer());
    assert!(screen.contains_text("Entry 2/2"));
    assert!(screen.contains_text("Testing a GraphQL Result<T>."));
    server.abort();
}

#[tokio::test]
async fn reader_passes_the_final_url_to_the_converter_and_displays_its_error() {
    use axum::{
        Router,
        response::{Html, Redirect},
        routing::get,
    };
    use synd_support::html::{Article, ArticleRequest, Excerpt, ExcerptRequest, HtmlConvError};
    use tokio::net::TcpListener;

    struct FailingConverter;
    impl HtmlConverter for FailingConverter {
        fn excerpt(&self, _: ExcerptRequest<'_>) -> Result<Excerpt, HtmlConvError> {
            unreachable!()
        }

        fn extract_article(&self, request: ArticleRequest<'_>) -> Result<Article, HtmlConvError> {
            assert_eq!(request.base_url().path(), "/articles/final");
            assert_eq!(request.html(), "<p>Publisher body</p>");
            Err(HtmlConvError::Conversion(
                std::io::Error::other("publisher conversion failed").into(),
            ))
        }
    }

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut entries = timeline_entries();
    entries.nodes[1].entry.website_url =
        Some(format!("http://{}/link", listener.local_addr().unwrap()));
    let publisher = Router::new()
        .route(
            "/link",
            get(|| async { Redirect::temporary("/articles/final") }),
        )
        .route(
            "/articles/final",
            get(|| async { Html("<p>Publisher body</p>") }),
        );
    let server = tokio::spawn(async move { axum::serve(listener, publisher).await.unwrap() });

    let (_cache_dir, mut app) = start_app_with_entries(entries, Some(Arc::new(FailingConverter)));
    let (tx, mut input) = event_stream();
    app.wait_until_jobs_completed(&mut input).await;
    tx.send_multi([key!('j'), key!(' ')]);
    app.wait_until_jobs_completed(&mut input).await;
    assert!(Screen::new(app.buffer()).contains_text("publisher conversion failed"));
    server.abort();
}

#[tokio::test]
async fn feeds_load_while_github_requests_wait_for_a_slot() {
    use std::{convert::Infallible, time::Duration};
    use tokio::sync::{mpsc, oneshot};

    let (started, mut requests) = mpsc::unbounded_channel();
    let service = tower::service_fn(move |_| {
        let (respond, response) = oneshot::channel::<()>();
        started.send(respond).unwrap();
        async move {
            response.await.unwrap();
            Ok::<_, Infallible>(http::Response::new(octocrab::OctoBody::from("[]")))
        }
    });
    let octocrab = octocrab::OctocrabBuilder::new_empty()
        .with_service(service)
        .with_auth(octocrab::AuthState::None)
        .build()
        .unwrap();
    let api = MockFeedApi::new([MockFeedApiResponse::Subscription(Ok(subscription()))]);
    let (_cache_dir, mut app) = app(api, Some(GhClient::with(octocrab)), None);
    let (tx, mut input) = event_stream();
    tx.send_multi((0..91).map(|_| key!('r')));

    tokio::time::timeout(Duration::from_secs(10), async {
        let (feed, mut running) = tokio::join!(
            app.run_until(&mut input, |buf| Screen::new(buf).contains_text("Feed 1/1")),
            async {
                let mut running = Vec::new();
                for _ in 0..90 {
                    running.push(requests.recv().await.unwrap());
                }
                tx.send_multi([key!(tab), key!(tab)]);
                running
            }
        );
        feed.unwrap();
        assert!(matches!(requests.try_recv(), Err(mpsc::error::TryRecvError::Empty)));

        running.pop().unwrap().send(()).unwrap();
        tokio::join!(app.wait_until_jobs_completed(&mut input), async {
            requests.recv().await.unwrap().send(()).unwrap();
            for request in running {
                request.send(()).unwrap();
            }
        });
    })
    .await
    .expect("Feeds must load with all GitHub slots held, and freeing one slot must start the waiting request");
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
    start_app_with_entries(timeline_entries(), None)
}

fn start_app_with_entries(
    entries: payload::TimelineEntryConnection,
    converter: Option<Arc<dyn HtmlConverter>>,
) -> (TempDir, Application) {
    let timeline_id = payload::TimelineId::from_u128(1);
    let api = MockFeedApi::new([
        MockFeedApiResponse::FeedEvents(Ok(Vec::new())),
        MockFeedApiResponse::Subscription(Ok(subscription())),
        MockFeedApiResponse::Timelines(Ok(Vec::new())),
        MockFeedApiResponse::CreateTimeline(
            "all".into(),
            Ok(payload::CreateTimelinePayload { timeline_id }),
        ),
        MockFeedApiResponse::TimelineEntries(timeline_id, Ok(entries)),
        MockFeedApiResponse::TimelineChanges(
            timeline_id,
            Ok(payload::TimelineChangesPayload {
                changes: Vec::new(),
                seq: 2,
                has_more: false,
            }),
        ),
    ]);
    let (cache_dir, mut app) = app(api, None, converter);

    app.bootstrap_for_test();

    (cache_dir, app)
}

fn app(
    feed_api: MockFeedApi,
    github: Option<GhClient>,
    converter: Option<Arc<dyn HtmlConverter>>,
) -> (TempDir, Application) {
    let cache_dir = tempfile::tempdir().expect("temp cache dir");
    let terminal = new_test_terminal(120, 30);
    let builder = Application::builder()
        .terminal(terminal)
        .feed_api(feed_api)
        .categories(Categories::default_toml())
        .config(Config {
            features: Features {
                enable_gh_notification: github.is_some(),
            },
            ..Config::default()
        })
        .cache(Cache::new(cache_dir.path().to_path_buf()))
        .theme(Theme::default())
        .interactor(Box::new(MockInteractor::new()));
    let builder = match converter {
        Some(converter) => builder.html_converter(converter),
        None => builder,
    };
    let app = match github {
        Some(client) => builder.gh_client(client).build(),
        None => builder.build(),
    };

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
                    "excerpt": { "text": "A note about feed architecture." },
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
                    "excerpt": { "text": "Testing a GraphQL Result<T>." },
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
