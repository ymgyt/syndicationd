#[cfg(feature = "integration")]
mod test {
    mod helper;

    use std::time::Duration;

    use crossterm::event::{Event, KeyCode, KeyEvent};
    use serial_test::file_serial;

    use synd_auth::device_flow::{provider, DeviceFlow};

    use synd_term::{
        application::{Application, Authenticator, Config, DeviceFlows},
        client::Client,
        config::Categories,
        ui::theme::Theme,
    };
    use tokio::net::TcpListener;
    use tokio_stream::wrappers::UnboundedReceiverStream;
    use tracing_subscriber::EnvFilter;

    #[tokio::test(flavor = "multi_thread")]
    #[file_serial(a)]
    async fn hello_world() -> anyhow::Result<()> {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .with_line_number(true)
            .with_file(true)
            .with_target(false)
            .init();

        tracing::info!("TEST hello_world run");

        let mock_port = 6000;
        let api_port = 6001;
        let oauth_addr = ("127.0.0.1", mock_port);
        let oauth_listener = TcpListener::bind(oauth_addr).await?;
        tokio::spawn(synd_test::mock::serve(oauth_listener));
        helper::serve_api(mock_port, api_port).await?;

        let endpoint = format!("https://localhost:{api_port}/graphql")
            .parse()
            .unwrap();
        let terminal = helper::new_test_terminal(120, 30);
        let client = Client::new(endpoint, Duration::from_secs(30)).unwrap();
        let device_flows = DeviceFlows {
            github: DeviceFlow::new(
                provider::Github::new("dummy")
                    .with_device_authorization_endpoint(format!(
                        "http://localhost:{mock_port}/case1/github/login/device/code",
                    ))
                    .with_token_endpoint(
                        "http://localhost:6000/case1/github/login/oauth/access_token",
                    ),
            ),
            google: DeviceFlow::new(provider::Google::new("dummy", "dummy")),
        };
        let authenticator = Authenticator::new().with_device_flows(device_flows);
        let config = Config {
            idle_timer_interval: Duration::from_millis(1000),
            throbber_timer_interval: Duration::from_secs(3600), // disable throbber
            ..Default::default()
        };
        // or mpsc and tokio_stream ReceiverStream
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let mut event_stream = UnboundedReceiverStream::new(rx);
        let theme = Theme::default();

        let mut application =
            Application::with(terminal, client, Categories::default_toml(), config)
                .with_theme(theme.clone())
                .with_authenticator(authenticator);
        application.event_loop_until_idle(&mut event_stream).await;

        insta::assert_debug_snapshot!(application.buffer());

        tracing::info!("Login assertion OK");

        // push enter => start auth flow
        let event = Event::Key(KeyEvent::from(KeyCode::Enter));
        tx.send(Ok(event)).unwrap();
        application.event_loop_until_idle(&mut event_stream).await;

        insta::assert_debug_snapshot!(application.buffer());

        tracing::info!("Login prompt assertion OK");

        // for quit event loop after polling job complete
        application.reset_idle_timer();
        // polling device access token complete
        application.event_loop_until_idle(&mut event_stream).await;

        Ok(())
    }
}
