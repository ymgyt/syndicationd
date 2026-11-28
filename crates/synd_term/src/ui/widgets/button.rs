use ratatui::text::{Line, Span};

use crate::ui::theme::Theme;

/// A focusable action, drawn in brackets in a footer or as plain text inline.
pub(crate) struct Button<'a> {
    label: &'a str,
    focused: bool,
    theme: &'a Theme,
    inline: bool,
}

impl<'a> Button<'a> {
    pub(crate) fn new(label: &'a str, focused: bool, theme: &'a Theme) -> Self {
        Self {
            label,
            focused,
            theme,
            inline: false,
        }
    }

    pub(crate) fn inline(mut self) -> Self {
        self.inline = true;
        self
    }
}

impl<'a> From<Button<'a>> for Line<'a> {
    fn from(button: Button<'a>) -> Self {
        let style = button.theme.tabs_selected;
        let style = if button.focused {
            style.reversed()
        } else if button.inline {
            style.not_bold()
        } else {
            style
        };
        if button.inline {
            Span::styled(button.label, style).into()
        } else {
            Span::styled(format!("[ {} ]", button.label), style).into()
        }
    }
}
