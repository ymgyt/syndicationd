use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::Modifier,
    widgets::{Row, StatefulWidget, TableState},
};

use crate::ui::{self, theme::EntriesTheme};

pub(crate) struct TableBuilder<H, R, C, T, S, M> {
    header: H,
    rows: R,
    widths: C,
    theme: T,
    selected_idx: S,
    highlight_modifier: M,
}

impl Default for TableBuilder<(), (), (), (), (), ()> {
    fn default() -> Self {
        Self {
            header: (),
            rows: (),
            widths: (),
            theme: (),
            selected_idx: (),
            highlight_modifier: (),
        }
    }
}

impl<R, C, T, S, M> TableBuilder<(), R, C, T, S, M> {
    pub(crate) fn header(self, header: Row<'_>) -> TableBuilder<Row<'_>, R, C, T, S, M> {
        TableBuilder {
            header,
            rows: self.rows,
            widths: self.widths,
            theme: self.theme,
            selected_idx: self.selected_idx,
            highlight_modifier: self.highlight_modifier,
        }
    }
}

impl<H, C, T, S, M> TableBuilder<H, (), C, T, S, M> {
    pub(crate) fn rows<'a, Rows>(self, rows: Rows) -> TableBuilder<H, Vec<Row<'a>>, C, T, S, M>
    where
        Rows: IntoIterator,
        Rows::Item: Into<Row<'a>>,
    {
        TableBuilder {
            header: self.header,
            rows: rows.into_iter().map(Into::into).collect(),
            widths: self.widths,
            theme: self.theme,
            selected_idx: self.selected_idx,
            highlight_modifier: self.highlight_modifier,
        }
    }
}

impl<H, R, T, S, M> TableBuilder<H, R, (), T, S, M> {
    pub(crate) fn widths<C>(self, widths: C) -> TableBuilder<H, R, Vec<Constraint>, T, S, M>
    where
        C: IntoIterator,
        C::Item: Into<Constraint>,
    {
        TableBuilder {
            header: self.header,
            rows: self.rows,
            widths: widths.into_iter().map(Into::into).collect(),
            theme: self.theme,
            selected_idx: self.selected_idx,
            highlight_modifier: self.highlight_modifier,
        }
    }
}

impl<H, R, C, S, M> TableBuilder<H, R, C, (), S, M> {
    pub(crate) fn theme(self, theme: &EntriesTheme) -> TableBuilder<H, R, C, &EntriesTheme, S, M>
    where
        C: IntoIterator,
        C::Item: Into<Constraint>,
    {
        TableBuilder {
            header: self.header,
            rows: self.rows,
            widths: self.widths,
            theme,
            selected_idx: self.selected_idx,
            highlight_modifier: self.highlight_modifier,
        }
    }
}

impl<H, R, C, T, M> TableBuilder<H, R, C, T, (), M> {
    pub(crate) fn selected_idx(self, selected_idx: usize) -> TableBuilder<H, R, C, T, usize, M> {
        TableBuilder {
            header: self.header,
            rows: self.rows,
            widths: self.widths,
            theme: self.theme,
            selected_idx,
            highlight_modifier: self.highlight_modifier,
        }
    }
}

impl<H, R, C, T, S> TableBuilder<H, R, C, T, S, ()> {
    pub(crate) fn highlight_modifier(
        self,
        highlight_modifier: Modifier,
    ) -> TableBuilder<H, R, C, T, S, Modifier> {
        TableBuilder {
            header: self.header,
            rows: self.rows,
            widths: self.widths,
            theme: self.theme,
            selected_idx: self.selected_idx,
            highlight_modifier,
        }
    }
}

impl<'a> TableBuilder<Row<'a>, Vec<Row<'a>>, Vec<Constraint>, &'a EntriesTheme, usize, Modifier> {
    pub(crate) fn build(self) -> Table<'a> {
        let TableBuilder {
            header,
            rows,
            widths,
            theme,
            selected_idx,
            highlight_modifier,
        } = self;

        let table = ratatui::widgets::Table::new(rows, widths)
            .header(header.style(theme.header))
            .column_spacing(2)
            .highlight_symbol(ui::TABLE_HIGHLIGHT_SYMBOL)
            .row_highlight_style(theme.selected_entry.add_modifier(highlight_modifier))
            .highlight_spacing(ratatui::widgets::HighlightSpacing::Always);

        let state = TableState::new().with_offset(0).with_selected(selected_idx);

        Table { table, state }
    }
}

pub(crate) struct Table<'a> {
    table: ratatui::widgets::Table<'a>,
    state: TableState,
}

impl<'a> Table<'a> {
    pub(crate) fn builder() -> TableBuilder<(), (), (), (), (), ()> {
        TableBuilder::default()
    }

    pub(crate) fn render(mut self, area: Rect, buf: &mut Buffer) {
        StatefulWidget::render(self.table, area, buf, &mut self.state);
    }
}
