#[cfg(feature = "integration")]
mod test {
    mod helper;

    use std::time::Duration;

    use crossterm::event::{Event, KeyCode, KeyEvent};
    use ratatui::{
        prelude::Buffer,
        style::{Modifier, Style},
    };
    use serial_test::file_serial;

    use synd_authn::device_flow::github::DeviceFlow;

    use synd_term::{
        application::{Application, Config},
        client::Client,
        ui::theme::Theme,
    };
    use tokio::net::TcpListener;
    use tokio_stream::wrappers::UnboundedReceiverStream;
    use tracing_subscriber::EnvFilter;

    #[tokio::test(flavor = "multi_thread")]
    #[file_serial(a)]
    async fn hello_world() -> anyhow::Result<()> {
        // TODO: wrap once
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

        let endpoint = format!("http://localhost:{api_port}/graphql")
            .parse()
            .unwrap();
        let terminal = helper::new_test_terminal();
        let client = Client::new(endpoint).unwrap();
        let config = Config {
            idle_timer_interval: Duration::from_millis(1000),
            throbber_timer_interval: Duration::from_secs(3600), // disable throbber
            github_device_flow: DeviceFlow::new("dummy")
                .with_device_authorization_endpoint(
                    "http://localhost:6000/case1/github/login/device/code",
                )
                .with_token_endpoint("http://localhost:6000/case1/github/login/oauth/access_token"),
        };
        // or mpsc and tokio_stream ReceiverStream
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let mut event_stream = UnboundedReceiverStream::new(rx);
        let theme = Theme::new();
        let bg = theme.background.bg.unwrap_or_default();

        let mut application = Application::with(terminal, client, config).with_theme(theme.clone());
        application.event_loop_until_idle(&mut event_stream).await;

        // login
        #[rustfmt::skip]
        let mut expected = Buffer::with_lines(vec![
            "                                                                                ",
            "                                                                                ",
            "                                                                                ",
            "                                                                                ",
            "                                                                                ",
            "                                      Login                                     ",
            "                        ────────────────────────────────                        ",
            "                        >> with GitHub                                          ",
            "                                                                                ",
            "                                                                                ",
            "                                                                                ",
            "                                                                                ",
            "                                                                                ",
            "                                                                                ",
            "                                                                                ",
            "                                                                                ",
            "                                                                                ",
            "                                                                                ",
            "                                                                                ",
            "                                                                                ",
        ]);
        for y in 0..expected.area.height {
            for x in 0..expected.area.width {
                expected.get_mut(x, y).set_bg(bg);
            }
        }
        // title
        for x in 38..43 {
            expected
                .get_mut(x, 5)
                .set_style(Style::new().add_modifier(Modifier::BOLD));
        }
        // auth provider
        for x in 24..56 {
            expected
                .get_mut(x, 7)
                .set_style(Style::new().add_modifier(Modifier::BOLD));
        }

        application.assert_buffer(&expected);

        tracing::info!("Login assertion OK");

        // push enter => start auth flow
        let event = Event::Key(KeyEvent::from(KeyCode::Enter));
        tx.send(Ok(event)).unwrap();
        application.event_loop_until_idle(&mut event_stream).await;

        // assert login prompt
        #[rustfmt::skip]
        let mut expected = Buffer::with_lines(vec![
            "                                                                                ",
            "                                                                                ",
            "                                                                                ",
            "                                                                                ",
            "                                                                                ",
            "                                      Login                                     ",
            "                        ────────────────────────────────                        ",
            "                        Open the following URL and Enter                        ",
            "                                                                                ",
            "                        URL:  https://syndicationd.ymgyt                        ",
            "                        Code: UC123456                                          ",
            "                                                                                ",
            "                                                                                ",
            "                                                                                ",
            "                                                                                ",
            "                                                                                ",
            "                                                                                ",
            "                                                                                ",
            "                                                                                ",
            "                                                                                ",
        ]);
        for y in 0..expected.area.height {
            for x in 0..expected.area.width {
                expected.get_mut(x, y).set_bg(bg);
            }
        }
        // title
        for x in 38..43 {
            expected
                .get_mut(x, 5)
                .set_style(Style::new().add_modifier(Modifier::BOLD));
        }
        // Bold url
        for x in 30..56 {
            expected
                .get_mut(x, 9)
                .set_style(Style::new().add_modifier(Modifier::BOLD));
        }
        // Bold code
        for x in 30..38 {
            expected
                .get_mut(x, 10)
                .set_style(Style::new().add_modifier(Modifier::BOLD));
        }

        application.assert_buffer(&expected);

        tracing::info!("Login prompt assertion OK");

        // for quit event loop after polling job complete
        application.reset_idle_timer();
        // polling device access token complete
        application.event_loop_until_idle(&mut event_stream).await;

        #[rustfmt::skip]
        let mut expected = Buffer::with_lines(vec![
            "    Syndicationd                                         Feeds    Subscription  ",
            "                                                                                ",
            "    Published   Title                                      Feed                 ",
            "                                                                                ",
            "                                                                                ",
            "                                                                                ",
            "                                                                                ",
            "                                                                                ",
            "                                                                                ",
            "                                                                                ",
            "                                                                                ",
            "                                                                                ",
            "                                                                                ",
            "                                                                                ",
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━Summary━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━",
            "                                                                                ",
            "                                                                                ",
            "                                                                                ",
            "                                                                                ",
            "              q  Quit  Tab  Next Tab  j/k  Up/Down  Ent  Open Entry             "
        ]);
        for y in 0..expected.area.height {
            for x in 0..expected.area.width {
                expected.get_mut(x, y).set_bg(bg);
            }
        }
        // tab line
        {
            // syndicationd
            for x in 4..16 {
                expected.get_mut(x, 0).set_style(theme.application_title);
            }
            // feeds tab left padding
            for x in 53..57 {
                expected.get_mut(x, 0).set_style(theme.tabs);
            }
            // selected feeds tab
            for x in 57..62 {
                expected.get_mut(x, 0).set_style(theme.tabs_selected);
            }
            // left padding and subscription
            for x in 62..78 {
                expected.get_mut(x, 0).set_style(theme.tabs);
            }
        }
        // table header
        {
            for x in 1..expected.area.width - 1 {
                expected.get_mut(x, 2).set_style(theme.entries.header);
            }
        }
        // prompt
        {
            // q
            for x in 13..16 {
                expected.get_mut(x, 19).set_style(theme.prompt.key);
            }
            // Quit
            for x in 16..22 {
                expected.get_mut(x, 19).set_style(theme.prompt.key_desc);
            }
            // Tab
            for x in 22..27 {
                expected.get_mut(x, 19).set_style(theme.prompt.key);
            }
            // Next Tab
            for x in 27..37 {
                expected.get_mut(x, 19).set_style(theme.prompt.key_desc);
            }
            // j/k
            for x in 37..42 {
                expected.get_mut(x, 19).set_style(theme.prompt.key);
            }
            // Up/Down
            for x in 42..51 {
                expected.get_mut(x, 19).set_style(theme.prompt.key_desc);
            }
            // Ent
            for x in 51..56 {
                expected.get_mut(x, 19).set_style(theme.prompt.key);
            }
            // Open entry
            for x in 56..68 {
                expected.get_mut(x, 19).set_style(theme.prompt.key_desc);
            }
        }

        application.assert_buffer(&expected);

        Ok(())
    }
}
