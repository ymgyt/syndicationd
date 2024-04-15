use ratatui::{style::Style, text::Span};
use synd_feed::types::Requirement;

use crate::ui::theme::RequirementLabelTheme;

pub trait RequirementExt {
    fn label(&self, theme: &RequirementLabelTheme) -> [Span<'static>; 3];
}

impl RequirementExt for Requirement {
    fn label(&self, theme: &RequirementLabelTheme) -> [Span<'static>; 3] {
        let (label, color) = match self {
            Requirement::Must => ("MST", theme.must),
            Requirement::Should => ("SHD", theme.should),
            Requirement::May => ("MAY", theme.may),
        };
        [
            Span::styled("", Style::default().fg(color)),
            Span::styled(label, Style::default().bg(color).fg(theme.fg)),
            Span::styled("", Style::default().fg(color)),
        ]
    }
}
