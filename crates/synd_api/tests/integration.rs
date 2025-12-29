#[cfg(feature = "integration")]
mod test {

    #[tokio::test(flavor = "multi_thread")]
    async fn api_command_test() -> anyhow::Result<()> {
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
