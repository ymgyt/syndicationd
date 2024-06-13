#[cfg(feature = "integration")]
mod test {
    use std::path::{Path, PathBuf};

    use synd_term::{auth::Credential, key, shift};
    use synd_test::temp_dir;

    mod helper;
    use crate::test::helper::{resize_event, TestCase};

    #[tokio::test(flavor = "multi_thread")]
    async fn login_with_github() -> anyhow::Result<()> {
        helper::init_tracing();

        let test_case = TestCase {
            mock_port: 6000,
            synd_api_port: 6001,
            kvsd_port: 47379,
            terminal_col_row: (120, 30),
            device_flow_case: "case1",
            cache_dir: temp_dir().into_path(),
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
            application.event_loop_until_idle(&mut event_stream).await;
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
            tx.send_multi([shift!('t'), shift!('t'), shift!('t')]);
            application
                .wait_until_jobs_completed(&mut event_stream)
                .await;
        }

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn login_with_google() -> anyhow::Result<()> {
        helper::init_tracing();

        let test_case = TestCase {
            mock_port: 6010,
            synd_api_port: 6011,
            kvsd_port: 47389,
            terminal_col_row: (120, 30),
            device_flow_case: "case1",
            cache_dir: temp_dir().into_path(),
            ..Default::default()
        };
        let mut application = test_case.init_app().await?;
        let (tx, mut event_stream) = helper::event_stream();

        {
            // push enter => start auth flow
            // Select google then select
            tx.send_multi([key!('j'), key!(enter)]);
            application.event_loop_until_idle(&mut event_stream).await;
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
    async fn refresh_expired_google_jwt() -> anyhow::Result<()> {
        let (expired_jwt, expired_at) = synd_test::jwt::google_expired_jwt();
        let test_case = TestCase {
            mock_port: 6040,
            synd_api_port: 6041,
            kvsd_port: 6042,
            terminal_col_row: (120, 30),
            device_flow_case: "case1",
            cache_dir: temp_dir().into_path(),
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
    async fn subscribe_then_unsubscribe() -> anyhow::Result<()> {
        helper::init_tracing();

        let test_case = TestCase {
            mock_port: 6020,
            synd_api_port: 6021,
            kvsd_port: 47399,
            terminal_col_row: (120, 30),
            interactor_buffer_fn: Some(|case: &TestCase| {
                format!(
                    "should rust http://localhost:{mock_port}/feed/twir_atom",
                    mock_port = case.mock_port
                )
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
            // Unsubscribe popup
            tx.send(key!('d'));
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

        Ok(())
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn filter_entries() -> anyhow::Result<()> {
        helper::init_tracing();

        let test_case = TestCase {
            // this port is hard coded in fixtures
            mock_port: 6030,
            synd_api_port: 6031,
            kvsd_port: 47409,
            kvsd_root_dir: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/kvsd/20240609"),
            terminal_col_row: (120, 30),
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
        }

        // Filter by requirement
        {
            // Move back then, change requirement to MUST
            tx.send_multi([key!(tab), key!('h'), key!('h')]);
            application
                .wait_until_jobs_completed(&mut event_stream)
                .await;
            insta::with_settings!({
                description => "entris after changing requirement to must",
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
                description => "entris after enable category filter",
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
                description => "entris after keyword search",
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
    async fn resize_terminal_to_zero() -> anyhow::Result<()> {
        let (mut col, mut row) = (120, 30);
        let test_case = TestCase {
            mock_port: 6050,
            synd_api_port: 6051,
            kvsd_port: 6052,
            terminal_col_row: (col, row),
            ..Default::default()
        }
        .already_logined();

        let mut application = test_case.init_app().await?;
        let (tx, mut event_stream) = helper::event_stream();

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
    async fn cli_commands() -> anyhow::Result<()> {
        let test_case = TestCase {
            mock_port: 7000,
            synd_api_port: 7001,
            kvsd_port: 7002,
            cache_dir: temp_dir().into_path(),
            ..Default::default()
        }
        .already_logined();

        test_case.init_app().await?;

        check_command_test(test_case.synd_api_port);
        export_command_test(test_case.synd_api_port, &test_case.cache_dir);
        term_command_test(&test_case.cache_dir, &test_case.log_path);
        // Exec clean last
        clean_command_test(&test_case.cache_dir);

        Ok(())
    }

    fn check_command_test(api_port: u16) {
        let mut cmd = assert_cmd::Command::cargo_bin("synd").unwrap();

        cmd.args([
            "check",
            "--endpoint",
            &format!("https://localhost:{api_port}"),
        ])
        .assert()
        .success();

        cmd.arg("--format=json").assert().success();
    }

    fn export_command_test(api_port: u16, cache_dir: &Path) {
        let mut cmd = assert_cmd::Command::cargo_bin("synd").unwrap();

        cmd.args([
            "export",
            "--endpoint",
            &format!("https://localhost:{api_port}"),
            "--cache-dir",
            &cache_dir.display().to_string(),
        ])
        .assert()
        .success();

        cmd.arg("--print-schema").assert().success();
    }

    fn clean_command_test(cache_dir: &Path) {
        let mut cmd = assert_cmd::Command::cargo_bin("synd").unwrap();

        cmd.args(["clean", "--cache-dir", &cache_dir.display().to_string()])
            .assert()
            .success();
    }

    fn term_command_test(cache_dir: &Path, log_path: &Path) {
        let mut cmd = assert_cmd::Command::cargo_bin("synd").unwrap();

        // Nix do not allow to create log file in user directory
        cmd.args([
            "--dry-run",
            "--cache-dir",
            &cache_dir.display().to_string(),
            "--log",
            &log_path.display().to_string(),
        ])
        .assert()
        .success();
    }
}
