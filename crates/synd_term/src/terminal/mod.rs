use anyhow::Result;
use crossterm::{
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::Frame;
use std::io;

#[cfg(not(feature = "integration"))]
mod backend;
#[cfg(not(feature = "integration"))]
pub use backend::{new_backend, TerminalBackend};

#[cfg(feature = "integration")]
mod integration_backend;
#[cfg(feature = "integration")]
pub use integration_backend::{new_backend, Buffer, TerminalBackend};

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
    pub fn init(&mut self) -> Result<()> {
        terminal::enable_raw_mode()?;
        crossterm::execute!(io::stdout(), EnterAlternateScreen)?;

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
    pub fn restore(&mut self) -> Result<()> {
        Self::restore_backend()?;
        self.backend.show_cursor()?;
        Ok(())
    }

    fn restore_backend() -> Result<()> {
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
    pub fn assert_buffer(&self, expected: &Buffer) {
        self.backend.backend().assert_buffer(expected);
    }
}
