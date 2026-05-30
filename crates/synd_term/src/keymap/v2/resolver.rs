use super::{
    CommandId, KeyStroke, KeymapAction, KeymapCandidate, KeymapRuntime, Layer,
    compiled::CompiledBinding, compiled::TrieSearch,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LayerStack {
    layers: Vec<Layer>,
}

impl LayerStack {
    pub(crate) fn new(layers: impl IntoIterator<Item = Layer>) -> Self {
        Self {
            layers: layers.into_iter().collect(),
        }
    }

    fn iter_high_to_low(&self) -> impl Iterator<Item = Layer> + '_ {
        self.layers.iter().rev().copied()
    }
}

impl<const N: usize> From<[Layer; N]> for LayerStack {
    fn from(layers: [Layer; N]) -> Self {
        Self::new(layers)
    }
}

#[derive(Debug, Default)]
pub(crate) struct KeyResolver {
    pending: Vec<KeyStroke>,
}

impl KeyResolver {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn clear_pending(&mut self) {
        self.pending.clear();
    }

    pub(crate) fn resolve(
        &mut self,
        keymaps: &KeymapRuntime,
        layers: &LayerStack,
        key: KeyStroke,
    ) -> KeymapResult {
        if self.pending.is_empty() {
            self.resolve_first_key(keymaps, layers, key)
        } else {
            self.resolve_pending_key(keymaps, layers, key)
        }
    }

    fn resolve_first_key(
        &mut self,
        keymaps: &KeymapRuntime,
        layers: &LayerStack,
        key: KeyStroke,
    ) -> KeymapResult {
        let keys = [key];
        for layer in layers.iter_high_to_low() {
            match keymaps.search(layer, &keys) {
                TrieSearch::Matched(binding) => return matched(binding),
                TrieSearch::Pending(candidates) => {
                    self.pending.push(key);
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

    fn resolve_pending_key(
        &mut self,
        keymaps: &KeymapRuntime,
        layers: &LayerStack,
        key: KeyStroke,
    ) -> KeymapResult {
        let first = self.pending[0];
        let Some(layer) = layers
            .iter_high_to_low()
            .find(|layer| matches!(keymaps.search(*layer, &[first]), TrieSearch::Pending(_)))
        else {
            let keys = std::mem::take(&mut self.pending);
            return KeymapResult::Cancelled { keys };
        };

        self.pending.push(key);
        match keymaps.search(layer, &self.pending) {
            TrieSearch::Matched(binding) => {
                self.pending.clear();
                matched(binding)
            }
            TrieSearch::Pending(candidates) => KeymapResult::Pending {
                keys: self.pending.clone(),
                candidates,
            },
            TrieSearch::NotFound => {
                let keys = std::mem::take(&mut self.pending);
                KeymapResult::Cancelled { keys }
            }
        }
    }
}

fn matched(binding: CompiledBinding) -> KeymapResult {
    match binding.action {
        KeymapAction::NoOp => KeymapResult::NoOp,
        action => KeymapResult::Matched(action),
    }
}

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
    NoOp,
}

impl From<CommandId> for KeymapResult {
    fn from(command: CommandId) -> Self {
        match KeymapAction::from(command) {
            KeymapAction::NoOp => Self::NoOp,
            action => Self::Matched(action),
        }
    }
}
