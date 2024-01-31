use ratatui::backend::TestBackend;

pub type Buffer = ratatui::buffer::Buffer;

pub type TerminalBackend = TestBackend;

pub fn new_backend() -> TerminalBackend {
    TestBackend::new(10, 10)
}
