#[cfg(feature = "integration")]
mod test {
    use std::sync::Once;

    static INIT: Once = Once::new();

    fn ensure_init() {
        INIT.call_once(|| {
            // Initialize rustls crypto provider for all integration tests
            rustls::crypto::ring::default_provider()
                .install_default()
                .unwrap();
        });
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn api_command_test() -> anyhow::Result<()> {
        ensure_init();
        let _kvsd_client = synd_test::kvsd::run_kvsd(
            "localhost".into(),
            45000,
            "test".into(),
            "test".into(),
            synd_test::temp_dir().keep(),
        )
        .await?;

        #[expect(deprecated)]
        let mut cmd = assert_cmd::Command::cargo_bin("synd-api").unwrap();

        cmd.args([
            "--addr",
            "127.0.0.1",
            "--port",
            &format!("{}", 45001),
            "--sqlite-db",
            &format!("{}", synd_test::temp_dir().keep().join("synd.db").display(),),
            "--tls-cert",
            synd_test::certificate().to_str().unwrap(),
            "--tls-key",
            synd_test::private_key().to_str().unwrap(),
            "--otlp-endpoint",
            "http://localhost:43177",
            "--dry-run",
        ])
        .assert()
        .success();

        Ok(())
    }
}
