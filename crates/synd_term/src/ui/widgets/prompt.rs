use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Stylize,
    text::{Line, Span},
    widgets::Widget,
};
use unicode_segmentation::{GraphemeCursor, UnicodeSegmentation};

use crate::command::PromptAction;

#[derive(Debug, Default)]
pub(crate) struct Prompt {
    line: String,
    cursor: usize,
}

impl Prompt {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn line(&self) -> &str {
        self.line.as_str()
    }

    /// Returns whether the text changed.
    pub(crate) fn apply(&mut self, action: &PromptAction) -> bool {
        let before = self.line.len();
        match action {
            PromptAction::InsertChar(ch) => self.insert_char(*ch),
            PromptAction::Paste(text) => self.insert_text(text),
            PromptAction::DeleteBackward => self.delete_backward(),
            PromptAction::DeleteForward => self.delete_forward(),
            PromptAction::Left => self.left(),
            PromptAction::Right => self.right(),
            PromptAction::Home => self.cursor = 0,
            PromptAction::End => self.cursor = self.line.len(),
        }
        before != self.line.len()
    }

    pub(crate) fn insert_char(&mut self, c: char) {
        self.line.insert(self.cursor, c);
        self.right();
    }

    fn insert_text(&mut self, text: &str) {
        let text: String = text.chars().filter(|ch| !ch.is_control()).collect();
        self.line.insert_str(self.cursor, &text);
        self.cursor += text.len();
        let mut cursor = GraphemeCursor::new(self.cursor, self.line.len(), true);
        if cursor.is_boundary(&self.line, 0) == Ok(false) {
            self.right();
        }
    }

    fn left(&mut self) {
        let mut cursor = GraphemeCursor::new(self.cursor, self.line.len(), true);
        self.cursor = cursor
            .prev_boundary(&self.line, 0)
            .ok()
            .flatten()
            .unwrap_or(0);
    }

    fn right(&mut self) {
        let mut cursor = GraphemeCursor::new(self.cursor, self.line.len(), true);
        self.cursor = cursor
            .next_boundary(&self.line, 0)
            .ok()
            .flatten()
            .unwrap_or(self.line.len());
    }

    fn delete_forward(&mut self) {
        let start = self.cursor;
        self.right();
        self.line.replace_range(start..self.cursor, "");
        self.cursor = start;
    }

    fn delete_backward(&mut self) {
        let end = self.cursor;
        self.left();
        self.line.replace_range(self.cursor..end, "");
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum RenderCursor {
    Enable,
    Disable,
}

impl Prompt {
    pub fn render(&self, area: Rect, buf: &mut Buffer, cursor: RenderCursor) {
        if cursor == RenderCursor::Disable {
            Line::raw(&self.line).render(area, buf);
            return;
        }
        let (before, after) = self.line.split_at(self.cursor);
        let mut after = after.graphemes(true);
        let caret = after.next().unwrap_or(" ");
        let mut width = usize::from(area.width).saturating_sub(Span::raw(caret).width());
        let mut start = before.len();
        for (index, grapheme) in before.grapheme_indices(true).rev() {
            let Some(remaining) = width.checked_sub(Span::raw(grapheme).width()) else {
                break;
            };
            width = remaining;
            start = index;
        }
        Line::from(vec![
            Span::raw(&before[start..]),
            Span::raw(caret).reversed(),
            Span::raw(after.as_str()),
        ])
        .render(area, buf);
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
