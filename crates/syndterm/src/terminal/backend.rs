use ratatui::backend::CrosstermBackend;

pub type TerminalBackend = CrosstermBackend<std::io::Stdout>;

pub fn new_backend() -> TerminalBackend {
    CrosstermBackend::new(std::io::stdout())
}
