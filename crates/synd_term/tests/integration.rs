#[cfg(feature = "integration")]
mod test {
    use std::path::Path;

    use synd_term::key;

    mod helper;
    use crate::test::helper::TestCase;

    #[tokio::test(flavor = "multi_thread")]
    async fn login() -> anyhow::Result<()> {
        helper::init_tracing();

        let test_case = TestCase {
            mock_port: 6000,
            synd_api_port: 6001,
            kvsd_port: 47379,
            terminal_col_row: (120, 30),
            device_flow_case: "case1",
            cache_dir: helper::temp_dir().into_path(),
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
            check_command_test(test_case.synd_api_port);
            export_command_test(test_case.synd_api_port, &test_case.cache_dir);
            clean_command_test(&test_case.cache_dir);
        }

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

    #[tokio::test(flavor = "multi_thread")]
    async fn subscribe_then_unsubscribe() -> anyhow::Result<()> {
        helper::init_tracing();

        let test_case = TestCase {
            mock_port: 6010,
            synd_api_port: 6011,
            kvsd_port: 47389,
            terminal_col_row: (120, 30),
            interactor_buffer: Some("should rust http://localhost:6010/feed/twir_atom".into()),
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
}
