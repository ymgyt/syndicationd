use std::collections::HashMap;

use crossterm::event::KeyEvent;

use super::{
    CommandId, CompiledKeymaps, KeyStroke, KeymapAction, KeymapCandidate, Layer, LayerKeymap,
    compiled::TrieSearch,
};

/// Active keymap layers ordered from low to high priority.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct LayerStack {
    layers: Vec<(Layer, bool)>,
}

impl LayerStack {
    pub(crate) fn extend(&mut self, layers: &[Layer]) {
        self.layers.retain(|(layer, _)| !layers.contains(layer));
        self.layers
            .extend(layers.iter().map(|&layer| (layer, false)));
    }

    pub(crate) fn cover(&mut self, layers: &[Layer]) {
        for (layer, covered) in &mut self.layers {
            if layers.contains(layer) {
                *covered = true;
            }
        }
    }

    pub(crate) fn truncate_after(&mut self, layer: Layer) {
        if let Some(index) = self.layers.iter().rposition(|(active, _)| *active == layer) {
            self.layers.truncate(index + 1);
        }
    }

    fn matched(&self, layer: Layer, action: KeymapAction) -> KeymapResult {
        let covered = self.layers.contains(&(layer, true));
        KeymapResult::Matched {
            layer,
            action: if covered {
                CommandId::Nop.into()
            } else {
                action
            },
        }
    }

    fn iter_high_to_low(&self) -> impl Iterator<Item = Layer> + '_ {
        self.layers.iter().rev().map(|(layer, _)| *layer)
    }
}

impl<const N: usize> From<[Layer; N]> for LayerStack {
    fn from(layers: [Layer; N]) -> Self {
        let mut stack = Self::default();
        stack.extend(&layers);
        stack
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
                TrieSearch::Matched(binding) => return layers.matched(layer, binding.action),
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
                layers.matched(layer, binding.action)
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

/// Outcome of resolving one key event through the current keymap state.
#[derive(Debug)]
pub(crate) enum KeymapResult {
    Matched {
        layer: Layer,
        action: KeymapAction,
    },
    Pending {
        keys: Vec<KeyStroke>,
        candidates: Vec<KeymapCandidate>,
    },
    NotFound,
    Cancelled {
        keys: Vec<KeyStroke>,
    },
}
