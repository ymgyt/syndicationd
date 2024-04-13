use ratatui::{
    style::{Color, Style},
    text::Span,
};
use synd_feed::types::Requirement;

pub trait RequirementExt {
    fn label(&self, color: Color) -> [Span<'static>; 3];
}

impl RequirementExt for Requirement {
    fn label(&self, fg: Color) -> [Span<'static>; 3] {
        let (label, color) = match self {
            Requirement::Must => ("MST", Color::Rgb(154, 4, 4)),
            Requirement::Should => ("SHD", Color::Rgb(243, 201, 105)),
            Requirement::May => ("MAY", Color::Rgb(35, 57, 91)),
        };
        [
            Span::styled("", Style::default().fg(color)),
            Span::styled(label, Style::default().bg(color).fg(fg)),
            Span::styled("", Style::default().fg(color)),
        ]
    }
}
