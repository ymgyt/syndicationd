use ratatui::{
    buffer::{Buffer, Cell},
    layout::Rect,
};

pub struct Screen<'a> {
    buffer: &'a Buffer,
}

impl<'a> Screen<'a> {
    #[must_use]
    pub const fn new(buffer: &'a Buffer) -> Self {
        Self { buffer }
    }

    #[must_use]
    pub fn lines(&self) -> Vec<String> {
        self.lines_in(*self.buffer.area())
    }

    #[must_use]
    pub fn lines_in(&self, area: Rect) -> Vec<String> {
        (area.top()..area.bottom())
            .map(|y| self.line_in(area, y))
            .collect()
    }

    #[must_use]
    pub fn contains_text(&self, text: &str) -> bool {
        self.lines().iter().any(|line| line.contains(text))
    }

    #[must_use]
    pub fn cell(&self, x: u16, y: u16) -> Option<&Cell> {
        self.buffer.cell((x, y))
    }

    fn line_in(&self, area: Rect, y: u16) -> String {
        let mut line = String::new();
        for x in area.left()..area.right() {
            line.push_str(self.cell(x, y).map_or(" ", Cell::symbol));
        }
        line.trim_end().to_owned()
    }
}
