use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{Paragraph, Widget as _, Wrap},
};
use synd_support::html::Article;

use super::Scroll;

/// Layout of one article at the last rendered width.
#[derive(Default)]
pub(crate) struct ArticleBody {
    width: u16,
    height: u16,
    lines: usize,
    top: u16,
    paragraph: Paragraph<'static>,
}

impl ArticleBody {
    pub(crate) fn scroll(&mut self, by: Scroll) {
        let half_page = (self.height / 2).max(1);
        self.top = match by {
            Scroll::LineUp => self.top.saturating_sub(1),
            Scroll::LineDown => self.top.saturating_add(1),
            Scroll::HalfPageUp => self.top.saturating_sub(half_page),
            Scroll::HalfPageDown => self.top.saturating_add(half_page),
            Scroll::PageUp => self.top.saturating_sub(self.height),
            Scroll::PageDown => self.top.saturating_add(self.height),
        }
        .min(self.last_top());
    }

    pub(crate) fn render(&mut self, article: &Article, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        if self.width != area.width {
            let lines = limner::render_markdown(
                article.markdown(),
                &limner::MarkdownStyle::default(),
                area.width,
            )
            .lines;
            self.paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
            self.lines = self.paragraph.line_count(area.width);
            self.width = area.width;
        }
        self.height = area.height;
        self.top = self.top.min(self.last_top());
        self.paragraph = std::mem::take(&mut self.paragraph).scroll((self.top, 0));
        (&self.paragraph).render(area, buf);
    }

    fn last_top(&self) -> u16 {
        u16::try_from(self.lines.saturating_sub(usize::from(self.height))).unwrap_or(u16::MAX)
    }
}
