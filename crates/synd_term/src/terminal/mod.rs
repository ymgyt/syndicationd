use crossterm::{
    ExecutableCommand,
    event::{EnableFocusChange, EventStream},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures_util::{Stream, future::Either, stream};
use ratatui::Frame;
use std::io::{self, IsTerminal};

#[cfg(not(feature = "integration"))]
mod backend;
#[cfg(not(feature = "integration"))]
pub use backend::{TerminalBackend, new_backend};

#[cfg(feature = "integration")]
mod integration_backend;
#[cfg(feature = "integration")]
pub use integration_backend::{Buffer, TerminalBackend, new_backend};

/// Provide terminal manipulation operations.
pub struct Terminal {
    backend: ratatui::Terminal<TerminalBackend>,
}

impl Terminal {
    /// Construct Terminal with default backend
    pub fn new() -> anyhow::Result<Self> {
        let backend = new_backend();
        Ok(Terminal::with(ratatui::Terminal::new(backend)?))
    }

    pub fn with(backend: ratatui::Terminal<TerminalBackend>) -> Self {
        Self { backend }
    }

    /// Initialize terminal
    pub fn init(&mut self) -> io::Result<()> {
        terminal::enable_raw_mode()?;
        crossterm::execute!(io::stdout(), EnterAlternateScreen, EnableFocusChange)?;

        let panic_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic| {
            Self::restore_backend().expect("Failed to reset terminal");
            panic_hook(panic);
        }));

        self.backend.hide_cursor()?;
        self.backend.clear()?;

        Ok(())
    }

    /// Reset terminal
    pub fn restore(&mut self) -> io::Result<()> {
        Self::restore_backend()?;
        self.backend.show_cursor()?;
        Ok(())
    }

    fn restore_backend() -> io::Result<()> {
        terminal::disable_raw_mode()?;
        io::stdout().execute(LeaveAlternateScreen)?;
        Ok(())
    }

    pub fn render<F>(&mut self, f: F) -> anyhow::Result<()>
    where
        F: FnOnce(&mut Frame),
    {
        self.backend.draw(f)?;
        Ok(())
    }

    pub fn force_redraw(&mut self) {
        self.backend.clear().unwrap();
    }

    #[cfg(feature = "integration")]
    pub fn buffer(&self) -> &Buffer {
        self.backend.backend().buffer()
    }
}

pub fn event_stream() -> impl Stream<Item = std::io::Result<crossterm::event::Event>> + Unpin {
    // When tests are run with nix(crane), /dev/tty is not available
    // In such cases, executing `EventStream::new()` will cause a panic.
    // Currently, this issue only arises during testing with nix, so an empty stream that does not panic is returned
    // https://github.com/crossterm-rs/crossterm/blob/fce58c879a748f3159216f68833100aa16141ab0/src/terminal/sys/file_descriptor.rs#L74
    // https://github.com/crossterm-rs/crossterm/blob/fce58c879a748f3159216f68833100aa16141ab0/src/event/read.rs#L39
    let is_terminal = std::io::stdout().is_terminal();

    if is_terminal {
        Either::Left(EventStream::new())
    } else {
        Either::Right(stream::empty())
    }
}
