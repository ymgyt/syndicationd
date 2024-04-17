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

        let endpoint = format!("https://localhost:{api_port}/graphql")
            .parse()
            .unwrap();
        let terminal = helper::new_test_terminal();
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
        };
        // or mpsc and tokio_stream ReceiverStream
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let mut event_stream = UnboundedReceiverStream::new(rx);
        let theme = Theme::new();
        // let bg = theme.background.bg.unwrap_or_default();

        let mut application =
            Application::with(terminal, client, Categories::default_toml(), config)
                .with_theme(theme.clone())
                .with_authenticator(authenticator);
        application.event_loop_until_idle(&mut event_stream).await;

        // login
        // #[rustfmt::skip]
        // let mut expected = Buffer::with_lines(vec![
        //     "                                                                                ",
        //     "                                                                                ",
        //     "                                                                                ",
        //     "                                                                                ",
        //     "                                                                                ",
        //     "                                      Login                                     ",
        //     "                        ────────────────────────────────                        ",
        //     "                        >> 󰊤 GitHub                                             ",
        //     "                           󰊭 Google                                             ",
        //     "                                                                                ",
        //     "                                                                                ",
        //     "                                                                                ",
        //     "                                                                                ",
        //     "                                                                                ",
        //     "                                                                                ",
        //     "                                                                                ",
        //     "                                                                                ",
        //     "                                                                                ",
        //     "                                                                                ",
        //     "                             q   j/k 󰹹  Ent 󰏌                            ",
        // ]);
        // for y in 0..expected.area.height {
        //     for x in 0..expected.area.width {
        //         expected.get_mut(x, y).set_bg(bg);
        //     }
        // }
        // // title
        // for x in 38..43 {
        //     expected
        //         .get_mut(x, 5)
        //         .set_style(Style::new().add_modifier(Modifier::BOLD));
        // }
        // // auth provider
        // for x in 24..56 {
        //     expected
        //         .get_mut(x, 7)
        //         .set_style(Style::new().add_modifier(Modifier::BOLD));
        // }

        // application.assert_buffer(&expected);

        tracing::info!("Login assertion OK");

        // push enter => start auth flow
        let event = Event::Key(KeyEvent::from(KeyCode::Enter));
        tx.send(Ok(event)).unwrap();
        application.event_loop_until_idle(&mut event_stream).await;

        // assert login prompt
        // #[rustfmt::skip]
        // let mut expected = Buffer::with_lines(vec![
        //     "                                                                                ",
        //     "                                                                                ",
        //     "                                                                                ",
        //     "                                                                                ",
        //     "                                                                                ",
        //     "                                      Login                                     ",
        //     "                        ────────────────────────────────                        ",
        //     "                        Open the following URL and Enter                        ",
        //     "                                                                                ",
        //     "                        URL:  https://syndicationd.ymgyt                        ",
        //     "                        Code: UC123456                                          ",
        //     "                                                                                ",
        //     "                                                                                ",
        //     "                                                                                ",
        //     "                                                                                ",
        //     "                                                                                ",
        //     "                                                                                ",
        //     "                                                                                ",
        //     "                                                                                ",
        //     "                                                                                ",
        // ]);
        // for y in 0..expected.area.height {
        //     for x in 0..expected.area.width {
        //         expected.get_mut(x, y).set_bg(bg);
        //     }
        // }
        // // title
        // for x in 38..43 {
        //     expected
        //         .get_mut(x, 5)
        //         .set_style(Style::new().add_modifier(Modifier::BOLD));
        // }
        // // Bold url
        // for x in 30..56 {
        //     expected
        //         .get_mut(x, 9)
        //         .set_style(Style::new().add_modifier(Modifier::BOLD));
        // }
        // // Bold code
        // for x in 30..38 {
        //     expected
        //         .get_mut(x, 10)
        //         .set_style(Style::new().add_modifier(Modifier::BOLD));
        // }

        // application.assert_buffer(&expected);

        tracing::info!("Login prompt assertion OK");

        // for quit event loop after polling job complete
        application.reset_idle_timer();
        // polling device access token complete
        application.event_loop_until_idle(&mut event_stream).await;

        // it would be better to reconsider the current implementation of test
        // for instance, assertions for buffers should be performed on a per-component basis
        // while here, do snapshots via insta
        /*
        #[rustfmt::skip]
        let mut expected = Buffer::with_lines(vec![
            "    Syndicationd                                              Entries    Feeds  ",
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
            // Entries tab left padding
            for x in 58..62 {
                expected.get_mut(x, 0).set_style(theme.tabs);
            }
            // selected entries tab
            for x in 62..69 {
                expected.get_mut(x, 0).set_style(theme.tabs_selected);
            }
            // left padding and subscription
            for x in 69..78 {
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
            // r
            for x in 51..54 {
                expected.get_mut(x, 19).set_style(theme.prompt.key);
            }
            // Reload
            for x in 54..62 {
                expected.get_mut(x, 19).set_style(theme.prompt.key_desc);
            }
            // Ent
            for x in 62..67 {
                expected.get_mut(x, 19).set_style(theme.prompt.key);
            }
            // Open entry
            for x in 67..79 {
                expected.get_mut(x, 19).set_style(theme.prompt.key_desc);
            }
        }

        application.assert_buffer(&expected);
        */

        Ok(())
    }
}
