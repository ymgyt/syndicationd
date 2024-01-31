use ratatui::backend::TestBackend;
use synd_term::terminal::Terminal;

pub fn new_test_terminal() -> Terminal {
    let backend = TestBackend::new(80, 20);
    let terminal = ratatui::Terminal::new(backend).unwrap();
    Terminal::with(terminal)
}
