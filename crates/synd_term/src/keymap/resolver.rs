use std::collections::HashMap;

use crossterm::event::KeyEvent;

use super::{
    CommandId, CompiledKeymaps, KeyStroke, KeymapAction, KeymapCandidate, Layer, LayerKeymap,
    compiled::CompiledBinding, compiled::TrieSearch,
};

const MAX_LAYER_STACK_LEN: usize = 8;

/// Active keymap layers ordered from low to high priority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LayerStack {
    layers: [Layer; MAX_LAYER_STACK_LEN],
    len: usize,
}

impl LayerStack {
    pub(crate) fn empty() -> Self {
        Self {
            layers: [Layer::App; MAX_LAYER_STACK_LEN],
            len: 0,
        }
    }

    pub(crate) fn new(layers: impl IntoIterator<Item = Layer>) -> Self {
        let mut stack = Self::empty();
        for layer in layers {
            stack.push(layer);
        }
        stack
    }

    pub(crate) fn push(&mut self, layer: Layer) {
        assert!(
            self.len < MAX_LAYER_STACK_LEN,
            "keymap layer stack capacity exceeded"
        );
        self.layers[self.len] = layer;
        self.len += 1;
    }

    fn as_slice(&self) -> &[Layer] {
        &self.layers[..self.len]
    }

    fn iter_high_to_low(&self) -> impl Iterator<Item = Layer> + '_ {
        self.as_slice().iter().rev().copied()
    }
}

impl<const N: usize> From<[Layer; N]> for LayerStack {
    fn from(layers: [Layer; N]) -> Self {
        Self::new(layers)
    }
}

/// Converts key events into keymap actions for the active layer stack.
#[derive(Debug)]
pub(crate) struct Keymap {
    static_keymaps: CompiledKeymaps,
    dynamic_keymaps: HashMap<Layer, LayerKeymap>,
    pending: Vec<KeyStroke>,
    pending_layers: Option<LayerStack>,
}

impl Keymap {
    pub(crate) fn new(static_keymaps: CompiledKeymaps) -> Self {
        Self {
            static_keymaps,
            dynamic_keymaps: HashMap::new(),
            pending: Vec::new(),
            pending_layers: None,
        }
    }

    pub(crate) fn default_keymaps() -> Self {
        Self::new(CompiledKeymaps::default_keymaps())
    }

    fn clear_pending(&mut self) {
        self.pending.clear();
        self.pending_layers = None;
    }

    pub(crate) fn sync_layers(&mut self, layers: &LayerStack) {
        if self
            .pending_layers
            .as_ref()
            .is_some_and(|pending| pending != layers)
        {
            self.clear_pending();
        }
    }

    pub(crate) fn set_layer_keymap(&mut self, keymap: LayerKeymap) {
        self.dynamic_keymaps.insert(keymap.layer(), keymap);
    }

    pub(crate) fn clear_layer_keymap(&mut self, layer: Layer) {
        self.dynamic_keymaps.remove(&layer);
    }

    pub(crate) fn resolve(&mut self, layers: &LayerStack, key: KeyEvent) -> KeymapResult {
        self.sync_layers(layers);
        self.resolve_stroke(layers, KeyStroke::from(key))
    }

    fn resolve_stroke(&mut self, layers: &LayerStack, key: KeyStroke) -> KeymapResult {
        if self.pending.is_empty() {
            self.resolve_first_key(layers, key)
        } else {
            self.resolve_pending_key(layers, key)
        }
    }

    fn resolve_first_key(&mut self, layers: &LayerStack, key: KeyStroke) -> KeymapResult {
        let keys = [key];
        for layer in layers.iter_high_to_low() {
            match self.search(layer, &keys) {
                TrieSearch::Matched(binding) => return matched(binding),
                TrieSearch::Pending(candidates) => {
                    self.pending.push(key);
                    self.pending_layers = Some(layers.clone());
                    return KeymapResult::Pending {
                        keys: self.pending.clone(),
                        candidates,
                    };
                }
                TrieSearch::NotFound => {}
            }
        }
        KeymapResult::NotFound
    }

    fn resolve_pending_key(&mut self, layers: &LayerStack, key: KeyStroke) -> KeymapResult {
        let first = self.pending[0];
        let Some(layer) = layers
            .iter_high_to_low()
            .find(|layer| matches!(self.search(*layer, &[first]), TrieSearch::Pending(_)))
        else {
            let keys = self.take_pending();
            return KeymapResult::Cancelled { keys };
        };

        self.pending.push(key);
        match self.search(layer, &self.pending) {
            TrieSearch::Matched(binding) => {
                self.clear_pending();
                matched(binding)
            }
            TrieSearch::Pending(candidates) => KeymapResult::Pending {
                keys: self.pending.clone(),
                candidates,
            },
            TrieSearch::NotFound => {
                let keys = self.take_pending();
                KeymapResult::Cancelled { keys }
            }
        }
    }

    fn take_pending(&mut self) -> Vec<KeyStroke> {
        self.pending_layers = None;
        std::mem::take(&mut self.pending)
    }

    fn search(&self, layer: Layer, keys: &[KeyStroke]) -> TrieSearch {
        if let Some(keymap) = self.dynamic_keymaps.get(&layer) {
            return keymap.search(keys);
        }
        self.static_keymaps
            .trie(layer)
            .map_or(TrieSearch::NotFound, |trie| trie.search(keys))
    }
}

impl Default for Keymap {
    fn default() -> Self {
        Self::default_keymaps()
    }
}

fn matched(binding: CompiledBinding) -> KeymapResult {
    KeymapResult::Matched(binding.action)
}

/// Outcome of resolving one key event through the current keymap state.
#[derive(Debug)]
pub(crate) enum KeymapResult {
    Matched(KeymapAction),
    Pending {
        keys: Vec<KeyStroke>,
        candidates: Vec<KeymapCandidate>,
    },
    NotFound,
    Cancelled {
        keys: Vec<KeyStroke>,
    },
}

impl From<CommandId> for KeymapResult {
    fn from(command: CommandId) -> Self {
        Self::Matched(KeymapAction::from(command))
    }
}
