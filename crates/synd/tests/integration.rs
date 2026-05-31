#[cfg(feature = "integration")]
mod test {
    use std::sync::Once;

    use synd_term::{
        application::{Config, Features},
        auth::Credential,
        key, shift,
    };
    use synd_test::temp_dir;

    mod helper;
    use crate::test::helper::{
        TestCase, focus_gained_event, focus_lost_event, resize_event, test_config,
    };

    static INIT: Once = Once::new();

    fn ensure_init() {
        INIT.call_once(|| {
            // Initialize rustls crypto provider for all integration tests
            let _ = rustls::crypto::ring::default_provider().install_default();
        });
    }

    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "timeline.entries is temporarily unavailable while timeline projection is redesigned"]
    async fn login_with_github() -> anyhow::Result<()> {
        helper::init_tracing();

        let test_case = TestCase {
            mock_port: 6000,
            synd_api_port: 6001,
            sqlite_db_id: 47379,
            terminal_col_row: (120, 30),
            device_flow_case: "case1",
            cache_dir: temp_dir().keep(),
            ..Default::default()
        };
        let mut application = test_case.init_app().await?;
        let (tx, mut event_stream) = helper::event_stream();

        {
            application
                .wait_until_jobs_completed(&mut event_stream)
                .await;
            insta::with_settings!({
                description => "initial login prompt",
            }, {
                insta::assert_debug_snapshot!("initial_login", application.buffer());
            });
        }

        {
            // push enter => start auth flow
            tx.send(key!(enter));
            let _ = application.event_loop_until_idle(&mut event_stream).await;
            insta::with_settings!({
                description => "show device flow code",
            },{
                insta::assert_debug_snapshot!("device_flow_prompt", application.buffer());
            });
        }

        {
            // polling device access token complete
            application
                .wait_until_jobs_completed(&mut event_stream)
                .await;
            insta::with_settings!({
                description => "initial landing entries",
            },{
                insta::assert_debug_snapshot!("landing_entries", application.buffer());
            });
        }

        {
            // Rotate theme
            tx.send_multi([
                shift!('t'),
                shift!('t'),
                shift!('t'),
                shift!('t'),
                shift!('t'),
            ]);
            application
                .wait_until_jobs_completed(&mut event_stream)
                .await;
        }

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "timeline.entries is temporarily unavailable while timeline projection is redesigned"]
    async fn login_with_google() -> anyhow::Result<()> {
        helper::init_tracing();

        let test_case = TestCase {
            mock_port: 6010,
            synd_api_port: 6011,
            sqlite_db_id: 47389,
            terminal_col_row: (120, 30),
            device_flow_case: "case1",
            cache_dir: temp_dir().keep(),
            ..Default::default()
        };
        let mut application = test_case.init_app().await?;
        let (tx, mut event_stream) = helper::event_stream();

        {
            // push enter => start auth flow
            // Select google then select
            tx.send_multi([key!('j'), key!(enter)]);
            let _ = application.event_loop_until_idle(&mut event_stream).await;
            insta::with_settings!({
                description => "show google device flow code",
            },{
                insta::assert_debug_snapshot!("google_device_flow_prompt", application.buffer());
            });
        }

        {
            // polling device access token complete
            application
                .wait_until_jobs_completed(&mut event_stream)
                .await;
            insta::with_settings!({
                description => "initial landing entries after google login",
            },{
                insta::assert_debug_snapshot!("google_landing_entries", application.buffer());
            });
        }
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "timeline.entries is temporarily unavailable while timeline projection is redesigned"]
    async fn refresh_expired_google_jwt() -> anyhow::Result<()> {
        ensure_init();
        let (expired_jwt, expired_at) = synd_test::jwt::google_expired_jwt();
        let test_case = TestCase {
            mock_port: 6040,
            synd_api_port: 6041,
            sqlite_db_id: 6042,
            terminal_col_row: (120, 30),
            device_flow_case: "case1",
            cache_dir: temp_dir().keep(),
            ..Default::default()
        }
        .with_credential(Credential::Google {
            id_token: expired_jwt,
            refresh_token: "dummy".into(),
            expired_at,
        });

        let mut application = test_case.init_app().await?;
        let (_tx, mut event_stream) = helper::event_stream();

        {
            application
                .wait_until_jobs_completed(&mut event_stream)
                .await;
            insta::with_settings!({
                description => "after_refreshing_expired_google_jwt",
            },{
                insta::assert_debug_snapshot!("refresh_expired_google_jwt_landing", application.buffer());
            });
        }

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "timeline.entries is temporarily unavailable while timeline projection is redesigned"]
    #[allow(clippy::too_many_lines)]
    async fn subscribe_then_unsubscribe() -> anyhow::Result<()> {
        helper::init_tracing();

        let test_case = TestCase {
            mock_port: 6020,
            synd_api_port: 6021,
            sqlite_db_id: 47399,
            terminal_col_row: (120, 30),
            interactor_buffer_fn: Some(|case: &TestCase| {
                vec![
                    format!(
                        "should rust http://localhost:{mock_port}/feed/twir_atom",
                        mock_port = case.mock_port
                    ),
                    // edit requirement from should to must
                    format!(
                        "must rust http://localhost:{mock_port}/feed/twir_atom",
                        mock_port = case.mock_port
                    ),
                    // internal error
                    format!(
                        "may rust http://localhost:{mock_port}/feed/error/internal",
                        mock_port = case.mock_port
                    ),
                    format!(
                        "may rust http://localhost:{mock_port}/feed/error/malformed",
                        mock_port = case.mock_port
                    ),
                ]
            }),
            ..Default::default()
        }
        .already_logined();

        let mut application = test_case.init_app().await?;
        let (tx, mut event_stream) = helper::event_stream();

        {
            // Move tab to feeds
            tx.send(key!(tab));
            application
                .wait_until_jobs_completed(&mut event_stream)
                .await;
            insta::with_settings!({
                description => "after feeds tab move",
            },{
                insta::assert_debug_snapshot!("subscribe_then_unsubscribe_landing_feeds", application.buffer());
            });
        }

        {
            // Subscribe
            tx.send(key!('a'));
            application
                .wait_until_jobs_completed(&mut event_stream)
                .await;
            insta::with_settings!({
                description => "after parsing editor buffer for subscribe",
            },{
                insta::assert_debug_snapshot!("subscribe_then_unsubscribe_after_editor_parse", application.buffer());
            });
        }

        {
            // Open feed. TODO: assert interactor
            tx.send(key!(enter));
            application
                .wait_until_jobs_completed(&mut event_stream)
                .await;
        }

        {
            // Edit feed
            tx.send(key!('e'));
            application
                .wait_until_jobs_completed(&mut event_stream)
                .await;
            insta::with_settings!({
                description => "after edit requirement from should to must",
            },{
                insta::assert_debug_snapshot!("subscribe_then_unsubscribe_after_edit", application.buffer());
            });
        }

        {
            // Prompt unsubscribe popup and move selection
            tx.send_multi([key!('d'), key!('l'), key!('h')]);
            application
                .wait_until_jobs_completed(&mut event_stream)
                .await;
            insta::with_settings!({
                description => "unsubscribe popup",
            },{
                insta::assert_debug_snapshot!("subscribe_then_unsubscribe_unsubscribe_popup", application.buffer());
            });
        }

        {
            // Select Yes (assuming Yes is selected)
            tx.send(key!(enter));
            application
                .wait_until_jobs_completed(&mut event_stream)
                .await;
            insta::with_settings!({
                description => "after unsubscribe",
            },{
                insta::assert_debug_snapshot!("subscribe_then_unsubscribe_unsubscribed", application.buffer());
            });
        }

        {
            // Handle the case that the server of the feed user tried to subscribe to is returning a internal error.
            tx.send(key!('a'));
            application
                .wait_until_jobs_completed(&mut event_stream)
                .await;
            insta::with_settings!({
                description => "handle internal error of the feed server",
            },{
                insta::assert_debug_snapshot!("subscribe_then_unsubscribe_feed_server_internal_error", application.buffer());
            });
        }

        {
            // Handle the case that the server of the feed user tried to subscribe to is returning a internal error.
            tx.send(key!('a'));
            application
                .wait_until_jobs_completed(&mut event_stream)
                .await;
            insta::with_settings!({
                description => "handle malformed xml error ",
            },{
                insta::assert_debug_snapshot!("subscribe_then_unsubscribe_malformed_xml_error", application.buffer());
            });
        }

        Ok(())
    }
    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "timeline.entries is temporarily unavailable while timeline projection is redesigned"]
    #[allow(clippy::too_many_lines)]
    async fn filter_entries() -> anyhow::Result<()> {
        helper::init_tracing();

        let test_case = TestCase {
            mock_port: 6030,
            synd_api_port: 6031,
            sqlite_db_id: 47409,
            sqlite_root_dir: temp_dir().keep(),
            terminal_col_row: (120, 30),
            config: Config {
                // To test pagination
                entries_per_pagination: 1,
                feeds_per_pagination: 1,
                ..test_config()
            },
            ..Default::default()
        }
        .already_logined();

        let mut application = test_case.init_app().await?;
        let (tx, mut event_stream) = helper::event_stream();

        // Initial fetch
        {
            application
                .wait_until_jobs_completed(&mut event_stream)
                .await;
            insta::with_settings!({
                description => "filter entries after initial fetch",
            },{
                insta::assert_debug_snapshot!("filter_entries_initial_fetch", application.buffer());
            });
            // Cover move
            tx.send_multi([key!('j'), key!('g'), key!('e'), key!('g'), key!('g')]);
        }

        // Move Feed tab
        {
            tx.send(key!(tab));
            application
                .wait_until_jobs_completed(&mut event_stream)
                .await;
            insta::with_settings!({
                description => "feed tab after initial fetch",
            },{
                insta::assert_debug_snapshot!("filter_entries_initial_fetch_feed", application.buffer());
            });
            // Cover move_last and move_first
            tx.send_multi([key!('g'), key!('e'), key!('g'), key!('g')]);
            // Move back
            tx.send(key!(tab));
        }

        {
            // Open entry. TODO: assert interactor
            tx.send(key!(enter));
            application
                .wait_until_jobs_completed(&mut event_stream)
                .await;
        }

        // Filter by requirement
        {
            // Change requirement to MUST
            tx.send_multi([key!('h'), key!('h')]);
            application
                .wait_until_jobs_completed(&mut event_stream)
                .await;
            insta::with_settings!({
                description => "entries after changing requirement to must",
            },{
                insta::assert_debug_snapshot!("filter_entries_req_must_entries", application.buffer());
            });
            // Change requirement to MAY
            tx.send_multi([key!('l'), key!('l')]);
        }

        // Filter by category
        {
            // Enable category filter and activate first category
            tx.send_multi([key!('c'), key!('-'), key!('a')]);
            application
                .wait_until_jobs_completed(&mut event_stream)
                .await;
            insta::with_settings!({
                description => "entries after enable category filter",
            },{
                insta::assert_debug_snapshot!("filter_entries_category_filter_entries", application.buffer());
            });
            // Enable all category
            tx.send_multi([key!('+'), key!(esc)]);
        }

        // Filter by keyword
        {
            // Enter keyword 'rust 549'
            tx.send_multi([
                key!('/'),
                key!('r'),
                key!('u'),
                key!('s'),
                key!('t'),
                key!(' '),
                key!('5'),
                key!('4'),
                key!('9'),
            ]);
            application
                .wait_until_jobs_completed(&mut event_stream)
                .await;
            insta::with_settings!({
                description => "entries after keyword search",
            },{
                insta::assert_debug_snapshot!("filter_entries_keyword_search_entries", application.buffer());
            });
            // Clear keyword
            tx.send_multi([
                key!(backspace),
                key!(backspace),
                key!(backspace),
                key!(backspace),
                key!(backspace),
                key!(backspace),
                key!(backspace),
                key!(backspace),
                key!(esc),
            ]);
        }
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn terminal_events() -> anyhow::Result<()> {
        ensure_init();
        let (mut col, mut row) = (120, 30);
        let test_case = TestCase {
            mock_port: 6050,
            synd_api_port: 6051,
            sqlite_db_id: 6052,
            terminal_col_row: (col, row),
            ..Default::default()
        }
        .already_logined();

        let mut application = test_case.init_app().await?;
        let (tx, mut event_stream) = helper::event_stream();

        // Focus
        {
            tx.send(focus_gained_event());
            tx.send(focus_lost_event());
            application
                .wait_until_jobs_completed(&mut event_stream)
                .await;
        }

        // Resize
        loop {
            col /= 2;
            row /= 2;
            if col == 0 && row == 0 {
                break;
            }
            // Assert that app do not panic
            tx.send(resize_event(col, row));
            application
                .wait_until_jobs_completed(&mut event_stream)
                .await;
        }
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn unauthorized() -> anyhow::Result<()> {
        ensure_init();
        let test_case = TestCase {
            mock_port: 6060,
            synd_api_port: 6061,
            sqlite_db_id: 6062,
            terminal_col_row: (120, 30),
            ..Default::default()
        }
        .with_credential(Credential::Github {
            // Use invalid token to mock unauthorized
            access_token: synd_test::GITHUB_INVALID_TOKEN.to_owned(),
        });

        let mut application = test_case.init_app().await?;
        let (_tx, mut event_stream) = helper::event_stream();

        // Assert unauthorized error message
        {
            application
                .wait_until_jobs_completed(&mut event_stream)
                .await;
            insta::with_settings!({
                description => "unauthorized error message(port is ignorable)",
            },{
                insta::assert_debug_snapshot!("unauthorized_error_message", application.buffer());
            });
        }

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "timeline.entries is temporarily unavailable while timeline projection is redesigned"]
    async fn github_notifications() -> anyhow::Result<()> {
        helper::init_tracing();

        let test_case = TestCase {
            mock_port: 6070,
            synd_api_port: 6071,
            sqlite_db_id: 6072,
            terminal_col_row: (120, 30),
            // Enable github notification features
            config: Config {
                features: Features {
                    enable_github_notification: true,
                },
                ..test_config()
            },
            now: Some(*synd_test::mock::github::notifications::NOW),
            ..Default::default()
        }
        .already_logined();

        let mut application = test_case.init_app().await?;
        let (tx, mut event_stream) = helper::event_stream();

        {
            application
                .wait_until_jobs_completed(&mut event_stream)
                .await;
            insta::with_settings!({
                description => "github notifications initial",
            },{
                insta::assert_debug_snapshot!("gh_notifications_init", application.buffer());
            });
        }

        {
            // Filter
            tx.send(key!('f'));
            // Enable private repo
            tx.send_multi([key!('p'), key!('r'), key!(enter)]);
            application
                .wait_until_jobs_completed(&mut event_stream)
                .await;
            insta::with_settings!({
                description => "github notifications enable private repo filter",
            },{
                insta::assert_debug_snapshot!("gh_notifications_filter_private_repo", application.buffer());
            });

            // Disable private and enable PR closed
            tx.send_multi([
                key!('f'),
                key!('p'),
                key!('r'),
                key!('c'),
                key!('l'),
                key!(enter),
            ]);
            application
                .wait_until_jobs_completed(&mut event_stream)
                .await;
            insta::with_settings!({
                description => "github notifications enable pr closed filter",
            },{
                insta::assert_debug_snapshot!("gh_notifications_filter_pr_closed", application.buffer());
            });

            // Disable PR closed and enable review requested
            tx.send_multi([
                key!('f'),
                key!('c'),
                key!('l'),
                key!('r'),
                key!('e'),
                key!(enter),
            ]);
            application
                .wait_until_jobs_completed(&mut event_stream)
                .await;
            insta::with_settings!({
                description => "github notifications enable review requested filter",
            },{
                insta::assert_debug_snapshot!("gh_notifications_filter_reason_review_requested", application.buffer());
            });

            // Clear filter conditions
            tx.send_multi([key!('f'), key!('r'), key!('e'), key!(enter)]);
        }

        {
            // Done
            tx.send(key!('d'));
            application
                .wait_until_jobs_completed(&mut event_stream)
                .await;

            // Unsubscribe
            tx.send(key!('u'));
            application
                .wait_until_jobs_completed(&mut event_stream)
                .await;

            // Done all
            tx.send(key!('D'));
            application
                .wait_until_jobs_completed(&mut event_stream)
                .await;

            insta::with_settings!({
                description => "github notifications mark as done all",
            },{
                insta::assert_debug_snapshot!("gh_notifications_mark_as_done_all", application.buffer());
            });
        }

        Ok(())
    }
}
