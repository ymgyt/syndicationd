use super::{KeySequence, KeymapAction, PromptAction};
use tuiframework::keymap::Binding;
pub(crate) type LayerKeymap = tuiframework::keymap::LayerKeymap<KeymapAction>;
pub(crate) fn search_prompt() -> LayerKeymap {
    let mut map = LayerKeymap::default();
    map.bind_input_chars(|ch| KeymapAction::Prompt(PromptAction::InsertChar(ch)));
    map.bind(Binding {
        on: KeySequence::parse(["backspace"]).expect("valid key"),
        command: KeymapAction::Prompt(PromptAction::DeleteBackward),
        desc: Some("Delete previous character".into()),
    })
    .expect("valid keymap");
    map
}

pub(super) fn line_editor() -> LayerKeymap {
    use PromptAction::{DeleteForward, End, Home, Left, Right};
    let mut map = search_prompt();
    for (key, action) in [
        ("delete", DeleteForward),
        ("left", Left),
        ("right", Right),
        ("home", Home),
        ("end", End),
    ] {
        map.bind(Binding {
            on: KeySequence::parse([key]).expect("valid editing key"),
            command: KeymapAction::Prompt(action),
            desc: None,
        })
        .expect("unique editing key");
    }
    map
}
