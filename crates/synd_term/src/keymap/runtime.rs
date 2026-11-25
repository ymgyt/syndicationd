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
