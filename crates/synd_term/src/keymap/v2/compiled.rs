use std::collections::{HashMap, HashSet};

use super::{
    CommandRegistry, KeyBinding, KeySequence, KeyStroke, KeymapAction, KeymapConfig, KeymapError,
    Layer, default::default_keymap_config,
};

#[derive(Clone, Debug)]
pub struct CompiledKeymaps {
    layers: HashMap<Layer, KeyTrie>,
    bindings: HashMap<Layer, Vec<KeyBinding>>,
}

impl CompiledKeymaps {
    pub(crate) fn compile(
        config: KeymapConfig,
        registry: &CommandRegistry,
    ) -> Result<Self, KeymapError> {
        let mut layers = HashMap::with_capacity(config.bindings.len());
        let mut compiled_bindings = HashMap::with_capacity(config.bindings.len());

        for (layer, bindings) in config.bindings {
            let mut seen = HashSet::with_capacity(bindings.len());
            let mut trie = KeyTrie::default();
            for binding in &bindings {
                registry.validate_binding(layer, binding)?;
                if !seen.insert(binding.on.clone()) {
                    return Err(KeymapError::DuplicateBinding {
                        layer,
                        keys: binding.on.clone(),
                    });
                }
                trie.insert(&ActionBinding::from(binding.clone()))?;
            }
            layers.insert(layer, trie);
            compiled_bindings.insert(layer, bindings);
        }

        Ok(Self {
            layers,
            bindings: compiled_bindings,
        })
    }

    pub fn default_keymaps() -> Self {
        Self::compile(default_keymap_config(), &CommandRegistry).expect("valid default keymap")
    }

    pub fn default_with_user_config(user_config: KeymapConfig) -> Result<Self, KeymapError> {
        let mut config = default_keymap_config();
        config.merge(user_config);
        Self::compile(config, &CommandRegistry)
    }

    pub(crate) fn bindings(&self, layer: Layer) -> &[KeyBinding] {
        self.bindings
            .get(&layer)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    pub(super) fn trie(&self, layer: Layer) -> Option<&KeyTrie> {
        self.layers.get(&layer)
    }
}

impl Default for CompiledKeymaps {
    fn default() -> Self {
        Self::default_keymaps()
    }
}

#[derive(Clone, Debug, Default)]
pub(super) struct KeyTrie {
    binding: Option<CompiledBinding>,
    children: HashMap<KeyStroke, KeyTrie>,
}

impl KeyTrie {
    pub(super) fn insert(&mut self, binding: &ActionBinding) -> Result<(), KeymapError> {
        let mut node = self;
        let mut prefix = Vec::with_capacity(binding.on.as_slice().len());
        for key in binding.on.as_slice() {
            prefix.push(*key);
            if node.binding.is_some() {
                return Err(KeymapError::AmbiguousBinding {
                    keys: KeySequence::new(prefix).expect("non-empty key sequence"),
                });
            }
            node = node.children.entry(*key).or_default();
        }
        if !node.children.is_empty() {
            return Err(KeymapError::AmbiguousBinding {
                keys: binding.on.clone(),
            });
        }
        node.binding = Some(CompiledBinding {
            action: binding.action.clone(),
            desc: binding.desc.clone(),
        });
        Ok(())
    }

    pub(super) fn search(&self, keys: &[KeyStroke]) -> TrieSearch {
        if keys.is_empty() {
            return TrieSearch::NotFound;
        }

        let mut node = self;
        for key in keys {
            let Some(next) = node.children.get(key) else {
                return TrieSearch::NotFound;
            };
            node = next;
        }

        if let Some(binding) = node.binding.clone() {
            TrieSearch::Matched(binding)
        } else if node.children.is_empty() {
            TrieSearch::NotFound
        } else {
            let candidates = node
                .children
                .iter()
                .map(|(key, child)| KeymapCandidate {
                    key: *key,
                    action: child.binding.as_ref().map(|binding| binding.action.clone()),
                    desc: child
                        .binding
                        .as_ref()
                        .and_then(|binding| binding.desc.clone()),
                })
                .collect();
            TrieSearch::Pending(candidates)
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct CompiledBinding {
    pub(super) action: KeymapAction,
    pub(super) desc: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct KeymapCandidate {
    pub(crate) key: KeyStroke,
    pub(crate) action: Option<KeymapAction>,
    pub(crate) desc: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ActionBinding {
    pub(super) on: KeySequence,
    pub(super) action: KeymapAction,
    pub(super) desc: Option<String>,
}

impl ActionBinding {
    pub(super) fn new(on: KeySequence, action: KeymapAction, desc: Option<String>) -> Self {
        Self { on, action, desc }
    }
}

impl From<KeyBinding> for ActionBinding {
    fn from(binding: KeyBinding) -> Self {
        Self {
            on: binding.on,
            action: KeymapAction::from(binding.command),
            desc: binding.desc,
        }
    }
}

pub(super) enum TrieSearch {
    Matched(CompiledBinding),
    Pending(Vec<KeymapCandidate>),
    NotFound,
}
