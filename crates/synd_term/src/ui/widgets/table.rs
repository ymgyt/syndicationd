use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Cell, Padding, Row, StatefulWidget, TableState},
};

use crate::{
    types::{Time, format_local},
    ui::{self, Context},
};

pub(super) const DATE_WIDTH: Constraint = Constraint::Length(5);

pub(super) fn title_cell<'a>(
    icon: Span<'a>,
    title: &'a str,
    selected: bool,
    cx: &Context<'_>,
) -> Cell<'a> {
    let title = if selected {
        Span::styled(title, cx.table_selection_style())
    } else {
        Span::raw(title)
    };
    Cell::from(Line::from(vec![icon, Span::raw(" "), title]))
}

pub(super) fn date_cell(time: Option<&Time>) -> Cell<'static> {
    let date = time.map_or_else(
        || ui::UNKNOWN_SYMBOL.to_owned(),
        |t| format_local(t, "%m/%d"),
    );
    Cell::from(Line::from(date).right_aligned())
}

pub(crate) struct TableBuilder<H, R, C, S> {
    header: H,
    rows: R,
    widths: C,
    selected_idx: S,
}

impl Default for TableBuilder<(), (), (), ()> {
    fn default() -> Self {
        Self {
            header: (),
            rows: (),
            widths: (),
            selected_idx: (),
        }
    }
}

impl<R, C, S> TableBuilder<(), R, C, S> {
    pub(crate) fn header(self, header: Row<'_>) -> TableBuilder<Row<'_>, R, C, S> {
        TableBuilder {
            header,
            rows: self.rows,
            widths: self.widths,
            selected_idx: self.selected_idx,
        }
    }
}

impl<H, C, S> TableBuilder<H, (), C, S> {
    pub(crate) fn rows<'a, Rows>(self, rows: Rows) -> TableBuilder<H, Vec<Row<'a>>, C, S>
    where
        Rows: IntoIterator,
        Rows::Item: Into<Row<'a>>,
    {
        TableBuilder {
            header: self.header,
            rows: rows.into_iter().map(Into::into).collect(),
            widths: self.widths,
            selected_idx: self.selected_idx,
        }
    }
}

impl<H, R, S> TableBuilder<H, R, (), S> {
    pub(crate) fn widths<C>(self, widths: C) -> TableBuilder<H, R, Vec<Constraint>, S>
    where
        C: IntoIterator,
        C::Item: Into<Constraint>,
    {
        TableBuilder {
            header: self.header,
            rows: self.rows,
            widths: widths.into_iter().map(Into::into).collect(),
            selected_idx: self.selected_idx,
        }
    }
}

impl<H, R, C> TableBuilder<H, R, C, ()> {
    pub(crate) fn selected_idx(self, selected_idx: usize) -> TableBuilder<H, R, C, usize> {
        TableBuilder {
            header: self.header,
            rows: self.rows,
            widths: self.widths,
            selected_idx,
        }
    }
}

impl<'a> TableBuilder<Row<'a>, Vec<Row<'a>>, Vec<Constraint>, usize> {
    pub(crate) fn build(self, cx: &Context<'_>) -> Table<'a> {
        let TableBuilder {
            header,
            rows,
            widths,
            selected_idx,
        } = self;

        let selected = cx.table_selection_style();
        let row_highlight = Style {
            fg: selected.fg,
            add_modifier: selected.add_modifier & Modifier::DIM,
            ..Style::new()
        };

        let table = ratatui::widgets::Table::new(rows, widths)
            .header(header.style(cx.theme.entries.header))
            .column_spacing(2)
            .row_highlight_style(row_highlight)
            .highlight_symbol(Span::styled(ui::TABLE_HIGHLIGHT_SYMBOL, selected))
            .highlight_spacing(ratatui::widgets::HighlightSpacing::Always);

        let state = TableState::new().with_offset(0).with_selected(selected_idx);

        Table { table, state }
    }
}

pub(crate) struct Table<'a> {
    table: ratatui::widgets::Table<'a>,
    state: TableState,
}

impl Table<'_> {
    /// Leave a row above the header and the rightmost cell for the scrollbar.
    pub(super) fn inner_area(area: Rect) -> Rect {
        Block::new().padding(Padding::new(0, 1, 1, 0)).inner(area)
    }

    pub(crate) fn builder() -> TableBuilder<(), (), (), ()> {
        TableBuilder::default()
    }

    pub(crate) fn render(mut self, area: Rect, buf: &mut Buffer) {
        StatefulWidget::render(self.table, area, buf, &mut self.state);
    }
}
