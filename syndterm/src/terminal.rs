use anyhow::Result;
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::{backend::CrosstermBackend, Frame};
use std::io;

type TerminalBackend = ratatui::Terminal<CrosstermBackend<io::Stdout>>;

/// Provide terminal manipulation operations.
pub struct Terminal {
    backend: TerminalBackend,
}

impl Terminal {
    pub fn new(backend: TerminalBackend) -> Self {
        Self { backend }
    }

    /// Construct Terminal from stdout
    pub fn from_stdout(out: io::Stdout) -> Result<Self> {
        let backend = CrosstermBackend::new(out);
        Ok(Terminal::new(ratatui::Terminal::new(backend)?))
    }

    /// Initialize terminal
    pub fn init(&mut self) -> Result<()> {
        terminal::enable_raw_mode()?;
        crossterm::execute!(io::stdout(), EnterAlternateScreen,)?;

        let panic_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic| {
            Self::reset_backend().expect("Failed to reset terminal");
            panic_hook(panic);
        }));

        self.backend.hide_cursor()?;
        self.backend.clear()?;

        Ok(())
    }

    /// Reset terminal
    pub fn exit(&mut self) -> Result<()> {
        Self::reset_backend()?;
        self.backend.show_cursor()?;
        Ok(())
    }

    fn reset_backend() -> Result<()> {
        terminal::disable_raw_mode()?;
        crossterm::execute!(io::stdout(), LeaveAlternateScreen,)?;
        Ok(())
    }

    pub fn render<F>(&mut self, f: F) -> anyhow::Result<()>
    where
        F: FnOnce(&mut Frame),
    {
        self.backend.draw(f)?;
        Ok(())
    }
}
