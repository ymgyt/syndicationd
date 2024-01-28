#[cfg(feature = "integration")]
mod test {
    mod helper;

    use crossterm::event::{Event, KeyCode, KeyEvent};
    use futures_util::stream;
    use ratatui::prelude::Buffer;
    use serial_test::file_serial;
    use syndterm::{application::Application, client::Client};

    #[tokio::test(flavor = "multi_thread")]
    #[file_serial(a)]
    async fn hello_world() -> anyhow::Result<()> {
        // TODO: wrap once
        tracing_subscriber::fmt::init();

        tracing::info!("TEST hello_world run");

        let endpoint = "http://localhost:5960".parse().unwrap();
        let terminal = helper::new_test_terminal();
        let client = Client::new(endpoint).unwrap();
        let mut application = Application::new(terminal, client);
        let _event = Event::Key(KeyEvent::from(KeyCode::Char('j')));
        // or mpsc and tokio_stream ReceiverStream
        let mut event_stream = stream::iter(vec![]);

        application.event_loop_until_idle(&mut event_stream).await;

        // login
        let expected = Buffer::with_lines(vec![
            "X                                                                               ",
            "                                                                                ",
            "                                                                                ",
            "                                                                                ",
            "                                                                                ",
            "                    Login                                                       ",
            "                    ────────────────────────────────────────                    ",
            "                    >> Github                                                   ",
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

        application.assert_buffer(&expected);

        Ok(())
    }
}
