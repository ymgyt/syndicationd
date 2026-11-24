//! Renders extracted articles for the terminal.

mod render;

pub(crate) use render::ArticleBody;

#[derive(Clone, Copy, Debug)]
pub(crate) enum Scroll {
    LineUp,
    LineDown,
    HalfPageUp,
    HalfPageDown,
    PageUp,
    PageDown,
}
