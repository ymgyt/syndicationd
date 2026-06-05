use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Stylize,
    text::{Line, Span},
    widgets::Widget,
};
use unicode_segmentation::GraphemeCursor;

#[derive(Debug, Clone, Copy)]
enum Move {
    BackwardChar(usize),
}

#[derive(Debug)]
pub(crate) struct Prompt {
    line: String,
    cursor: usize,
}

impl Prompt {
    pub fn new() -> Self {
        Self {
            line: String::new(),
            cursor: 0,
        }
    }

    pub fn line(&self) -> &str {
        self.line.as_str()
    }

    pub(crate) fn insert_char(&mut self, c: char) {
        self.line.insert(self.cursor, c);
        let mut cursor = GraphemeCursor::new(self.cursor, self.line.len(), true);
        if let Ok(Some(pos)) = cursor.next_boundary(&self.line, 0) {
            self.cursor = pos;
        }
    }

    pub(crate) fn delete_backward(&mut self) {
        let pos = self.move_cursor(Move::BackwardChar(1));
        self.line.replace_range(pos..self.cursor, "");
        self.cursor = pos;
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum RenderCursor {
    Enable,
    Disable,
}

impl Prompt {
    fn move_cursor(&self, m: Move) -> usize {
        match m {
            Move::BackwardChar(n) => {
                let mut position = self.cursor;
                for _ in 0..n {
                    let mut cursor = GraphemeCursor::new(position, self.line.len(), true);
                    if let Ok(Some(pos)) = cursor.prev_boundary(&self.line, 0) {
                        position = pos;
                    } else {
                        break;
                    }
                }
                position
            }
        }
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer, cursor: RenderCursor) {
        let mut spans = vec![Span::from(&self.line)];

        if cursor == RenderCursor::Enable {
            spans.push(Span::from(" ").reversed());
        }
        Line::from(spans).render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_ascii() {
        let mut p = Prompt::new();
        p.insert_char('a');
        p.insert_char('b');
        p.insert_char('c');
        assert_eq!(p.line(), "abc");
    }

    #[test]
    fn prompt_grapheme() {
        let mut p = Prompt::new();
        p.insert_char('山');
        p.insert_char('口');
        p.delete_backward();

        assert_eq!(p.line(), "山");

        p.delete_backward();
        assert_eq!(p.line(), "");

        p.delete_backward();
        p.delete_backward();
        p.delete_backward();
        assert_eq!(p.line(), "");
    }
}
