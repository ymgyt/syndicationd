// Currently throbber-widgets-tui dependes ratatui 0.24
// https://github.com/arkbig/throbber-widgets-tui/blob/cd6d1e1e1f38e221d8462df66172dcc370582bbd/Cargo.toml#L20

use ratatui::{
    prelude::{Buffer, Rect},
    style::Style,
    text::Span,
    widgets::StatefulWidget,
};

#[derive(Debug, Clone, Default)]
pub struct ThrobberState {
    index: i8,
}

impl ThrobberState {
    /// Get a index.
    pub fn index(&self) -> i8 {
        self.index
    }
    pub fn calc_next(&mut self) {
        self.calc_step(1);
    }

    pub fn calc_step(&mut self, step: i8) {
        self.index = self.index.saturating_add(step);
    }

    #[allow(clippy::cast_possible_truncation)]
    pub fn normalize(&mut self, throbber: &Throbber) {
        let len = throbber.throbber_set.symbols.len() as i8;
        if len <= 0 {
            //ng but it's not used, so it stays.
        } else {
            self.index %= len;
            if self.index < 0 {
                // Negative numbers are indexed from the tail
                self.index += len;
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Throbber<'a> {
    label: Option<Span<'a>>,
    style: Style,
    throbber_style: Style,
    throbber_set: throbber::Set,
    use_type: throbber::WhichUse,
}

impl<'a> Default for Throbber<'a> {
    fn default() -> Self {
        Self {
            label: None,
            style: Style::default(),
            throbber_style: Style::default(),
            throbber_set: throbber::BRAILLE_SIX,
            use_type: throbber::WhichUse::Spin,
        }
    }
}

impl<'a> Throbber<'a> {
    #[must_use]
    pub fn label<T>(mut self, label: T) -> Self
    where
        T: Into<Span<'a>>,
    {
        self.label = Some(label.into());
        self
    }

    #[must_use]
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    #[must_use]
    pub fn throbber_style(mut self, style: Style) -> Self {
        self.throbber_style = style;
        self
    }

    #[must_use]
    pub fn throbber_set(mut self, set: throbber::Set) -> Self {
        self.throbber_set = set;
        self
    }

    #[must_use]
    pub fn use_type(mut self, use_type: throbber::WhichUse) -> Self {
        self.use_type = use_type;
        self
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
impl<'a> StatefulWidget for Throbber<'a> {
    type State = ThrobberState;

    /// Render specified index symbols.
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        buf.set_style(area, self.style);

        let throbber_area = area;
        if throbber_area.height < 1 {
            return;
        }

        // render a symbol.
        let symbol = match self.use_type {
            throbber::WhichUse::Full => self.throbber_set.full,
            throbber::WhichUse::Empty => self.throbber_set.empty,
            throbber::WhichUse::Spin => {
                state.normalize(&self);
                let len = self.throbber_set.symbols.len() as i8;
                if 0 <= state.index && state.index < len {
                    self.throbber_set.symbols[state.index as usize]
                } else {
                    self.throbber_set.empty
                }
            }
        };
        let symbol_span = Span::styled(format!("{symbol} "), self.throbber_style);
        let (col, row) = buf.set_span(
            throbber_area.left(),
            throbber_area.top(),
            &symbol_span,
            symbol_span.width() as u16,
        );

        // render a label.
        if let Some(label) = self.label {
            if throbber_area.right() <= col {
                return;
            }
            buf.set_span(col, row, &label, label.width() as u16);
        }
    }
}

#[allow(clippy::module_inception, clippy::doc_link_with_quotes)]
pub mod throbber {
    /// A set of symbols to be rendered by throbber.
    #[derive(Debug, Clone)]
    pub struct Set {
        pub full: &'static str,
        pub empty: &'static str,
        pub symbols: &'static [&'static str],
    }

    /// Rendering object.
    ///
    /// If Spin is specified, ThrobberState.index is used.
    #[derive(Debug, Clone)]
    pub enum WhichUse {
        Full,
        Empty,
        Spin,
    }

    /// ["|", "/", "-", "\\"]
    pub const ASCII: Set = Set {
        full: "*",
        empty: " ",
        symbols: &["|", "/", "-", "\\"],
    };

    /// ["│", "╱", "─", "╲"]
    pub const BOX_DRAWING: Set = Set {
        full: "┼",
        empty: "　",
        symbols: &["│", "╱", "─", "╲"],
    };

    /// ["⠘", "⠰", "⠤", "⠆", "⠃", "⠉"]
    pub const BRAILLE_DOUBLE: Set = Set {
        full: "⠿",
        empty: "　",
        symbols: &["⠘", "⠰", "⠤", "⠆", "⠃", "⠉"],
    };

    /// ["⠷", "⠯", "⠟", "⠻", "⠽", "⠾"]
    pub const BRAILLE_SIX: Set = Set {
        full: "⠿",
        empty: "　",
        symbols: &["⠷", "⠯", "⠟", "⠻", "⠽", "⠾"],
    };

    /// ["⠧", "⠏", "⠛", "⠹", "⠼", "⠶"]
    pub const BRAILLE_SIX_DOUBLE: Set = Set {
        full: "⠿",
        empty: "　",
        symbols: &["⠧", "⠏", "⠛", "⠹", "⠼", "⠶"],
    };

    /// ["⣷", "⣯", "⣟", "⡿", "⢿", "⣻", "⣽", "⣾"]
    pub const BRAILLE_EIGHT: Set = Set {
        full: "⣿",
        empty: "　",
        symbols: &["⣷", "⣯", "⣟", "⡿", "⢿", "⣻", "⣽", "⣾"],
    };

    /// ["⣧", "⣏", "⡟", "⠿", "⢻", "⣹", "⣼", "⣶"]
    pub const BRAILLE_EIGHT_DOUBLE: Set = Set {
        full: "⣿",
        empty: "　",
        symbols: &["⣧", "⣏", "⡟", "⠿", "⢻", "⣹", "⣼", "⣶"],
    };
}
