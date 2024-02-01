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

        let oauth_addr = ("127.0.0.1", 6000);
        let oauth_listener = TcpListener::bind(oauth_addr).await?;
        tokio::spawn(synd_test::oauth::serve(oauth_listener));

        let endpoint = "http://localhost:5960".parse().unwrap();
        let terminal = helper::new_test_terminal();
        let client = Client::new(endpoint).unwrap();
        let config = Config {
            idle_timer_interval: Duration::from_millis(2000),
            github_device_flow: DeviceFlow::new("dummy")
                .with_endpoint("http://localhost:6000/github/login/device/code"),
        };
        // or mpsc and tokio_stream ReceiverStream
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let mut event_stream = UnboundedReceiverStream::new(rx);
        let theme = Theme::new();
        let bg = theme.background.bg.unwrap_or_default();

        let mut application = Application::with(terminal, client, config);
        application.event_loop_until_idle(&mut event_stream).await;

        // login
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

        Ok(())
    }
}
