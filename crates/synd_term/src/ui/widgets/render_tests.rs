use ratatui::{buffer::Buffer, layout::Rect, style::Modifier};

use crate::{
    test_support::screen::Screen,
    ui::widgets::prompt::{Prompt, RenderCursor},
};

#[test]
fn prompt_render_outputs_visible_text_and_cursor_cell_style() {
    let mut prompt = Prompt::new();
    prompt.insert_char('a');
    prompt.insert_char('b');
    prompt.insert_char('c');

    let area = Rect::new(0, 0, 8, 1);
    let mut buffer = Buffer::empty(area);
    prompt.render(area, &mut buffer, RenderCursor::Enable);

    let screen = Screen::new(&buffer);
    assert_eq!(screen.lines(), vec![String::from("abc")]);

    let cursor = screen.cell(3, 0).expect("cursor cell");
    assert_eq!(cursor.symbol(), " ");
    assert!(cursor.modifier.contains(Modifier::REVERSED));
}
