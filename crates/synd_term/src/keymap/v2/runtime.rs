use std::collections::HashMap;

use super::{
    CompiledKeymaps, KeySequence, KeyStroke, KeymapAction, KeymapError, Layer, PromptAction,
    compiled::{ActionBinding, CompiledBinding, KeyTrie, TrieSearch},
};

#[derive(Clone, Debug)]
pub(crate) struct KeymapRuntime {
    static_keymaps: CompiledKeymaps,
    dynamic_keymaps: HashMap<Layer, LayerKeymap>,
}

impl KeymapRuntime {
    pub(crate) fn new(static_keymaps: CompiledKeymaps) -> Self {
        Self {
            static_keymaps,
            dynamic_keymaps: HashMap::new(),
        }
    }

    pub(crate) fn default_keymaps() -> Self {
        Self::new(CompiledKeymaps::default_keymaps())
    }

    pub(crate) fn set_layer_keymap(&mut self, keymap: LayerKeymap) {
        self.dynamic_keymaps.insert(keymap.layer, keymap);
    }

    pub(crate) fn clear_layer_keymap(&mut self, layer: Layer) {
        self.dynamic_keymaps.remove(&layer);
    }

    pub(super) fn search(&self, layer: Layer, keys: &[KeyStroke]) -> TrieSearch {
        if let Some(keymap) = self.dynamic_keymaps.get(&layer) {
            return keymap.search(keys);
        }
        self.static_keymaps
            .trie(layer)
            .map_or(TrieSearch::NotFound, |trie| trie.search(keys))
    }
}

#[derive(Clone, Debug)]
pub(crate) struct LayerKeymap {
    layer: Layer,
    trie: KeyTrie,
    bindings: Vec<ActionBinding>,
    input_chars: bool,
}

impl LayerKeymap {
    pub(crate) fn builder(layer: Layer) -> LayerKeymapBuilder {
        LayerKeymapBuilder::new(layer)
    }

    fn search(&self, keys: &[KeyStroke]) -> TrieSearch {
        if self.input_chars
            && keys.len() == 1
            && let Some(ch) = keys[0].as_char()
        {
            return TrieSearch::Matched(CompiledBinding {
                action: KeymapAction::Prompt(PromptAction::InsertChar(ch)),
                desc: Some("Insert character".to_owned()),
            });
        }

        self.trie.search(keys)
    }

    pub(crate) fn search_prompt() -> Self {
        let mut builder = Self::builder(Layer::SearchPrompt);
        builder.bind_input_chars();
        builder
            .bind(
                ["backspace"],
                KeymapAction::Prompt(PromptAction::DeleteBackward),
                Some("Delete previous character"),
            )
            .expect("valid search prompt keymap");
        builder.build().expect("valid search prompt keymap")
    }
}

pub(crate) struct LayerKeymapBuilder {
    layer: Layer,
    bindings: Vec<ActionBinding>,
    input_chars: bool,
}

impl LayerKeymapBuilder {
    fn new(layer: Layer) -> Self {
        Self {
            layer,
            bindings: Vec::new(),
            input_chars: false,
        }
    }

    pub(crate) fn bind<const N: usize>(
        &mut self,
        keys: [&str; N],
        action: KeymapAction,
        desc: Option<&str>,
    ) -> Result<(), KeymapError> {
        let on = KeySequence::parse(keys)?;
        self.bind_sequence(on, action, desc.map(str::to_owned))
    }

    pub(crate) fn bind_key(
        &mut self,
        key: KeyStroke,
        action: KeymapAction,
        desc: Option<String>,
    ) -> Result<(), KeymapError> {
        self.bind_sequence(KeySequence::new(vec![key])?, action, desc)
    }

    pub(crate) fn bind_input_chars(&mut self) {
        self.input_chars = true;
    }

    pub(crate) fn build(self) -> Result<LayerKeymap, KeymapError> {
        let mut trie = KeyTrie::default();
        for binding in &self.bindings {
            trie.insert(binding)?;
        }

        Ok(LayerKeymap {
            layer: self.layer,
            trie,
            bindings: self.bindings,
            input_chars: self.input_chars,
        })
    }

    fn bind_sequence(
        &mut self,
        on: KeySequence,
        action: KeymapAction,
        desc: Option<String>,
    ) -> Result<(), KeymapError> {
        let binding = ActionBinding::new(on, action, desc);
        if self
            .bindings
            .iter()
            .any(|existing| existing.on == binding.on)
        {
            return Err(KeymapError::DuplicateBinding {
                layer: self.layer,
                keys: binding.on,
            });
        }
        self.bindings.push(binding);
        Ok(())
    }
}
