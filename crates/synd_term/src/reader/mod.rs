//! Extracts articles from publisher HTML and renders them for the terminal.

mod extract;
mod render;

pub(crate) use extract::extract;
pub(crate) use render::ArticleBody;

#[derive(Debug)]
pub(crate) struct Article {
    pub(crate) title: String,
    markdown: String,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Scroll {
    LineUp,
    LineDown,
    HalfPageUp,
    HalfPageDown,
    PageUp,
    PageDown,
}
