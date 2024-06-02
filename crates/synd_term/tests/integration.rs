#[cfg(feature = "integration")]
mod test {
    use std::path::Path;

    use serial_test::file_serial;
    use synd_term::key;
    use tokio_stream::wrappers::UnboundedReceiverStream;

    mod helper;
    use crate::test::helper::TestCase;

    #[tokio::test(flavor = "multi_thread")]
    #[file_serial(a)]
    async fn happy() -> anyhow::Result<()> {
        helper::init_tracing();

        let test_case = TestCase {
            oauth_provider_port: 6000,
            synd_api_port: 6001,
            kvsd_port: 47379,
            terminal_col_row: (120, 30),
            device_flow_case: "case1",
            cache_dir: helper::temp_dir().into_path(),
        };
        let mut application = test_case.init_app().await?;

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let mut event_stream = UnboundedReceiverStream::new(rx);

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
            tx.send(Ok(key!(enter))).unwrap();
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
}
