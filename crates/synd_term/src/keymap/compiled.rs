use indexmap::{IndexMap, map::Entry};
use std::collections::{HashMap, HashSet};

use super::{
    CommandRegistry, KeyBinding, KeySequence, KeyStroke, KeymapAction, KeymapConfig, KeymapError,
    Layer, default::default_keymap_config,
};

/// Validated static keymaps compiled into layer-specific tries.
#[derive(Clone, Debug)]
pub struct CompiledKeymaps {
    layers: HashMap<Layer, KeyTrie>,
}

impl CompiledKeymaps {
    pub(crate) fn compile(
        config: KeymapConfig,
        registry: &CommandRegistry,
    ) -> Result<Self, KeymapError> {
        let mut layers = HashMap::with_capacity(config.bindings.len());

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
        }

        if let Some(list) = layers.remove(&Layer::List) {
            for layer in [Layer::Entries, Layer::Feeds, Layer::GhNotifications] {
                if let Some(local) = layers.get_mut(&layer) {
                    // Once a local prefix matches, the resolver stays in its trie.
                    for (key, branch) in &mut local.children {
                        if let Some(shared) = list.children.get(key) {
                            branch.inherit(shared);
                        }
                    }
                }
            }
            layers.insert(Layer::List, list);
        }
        Ok(Self { layers })
    }

    pub fn default_keymaps() -> Self {
        Self::compile(default_keymap_config(), &CommandRegistry).expect("valid default keymap")
    }

    pub fn default_with_user_config(user_config: KeymapConfig) -> Result<Self, KeymapError> {
        let mut config = default_keymap_config();
        config.merge(user_config);
        Self::compile(config, &CommandRegistry)
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

/// Prefix tree used to resolve single-key and multi-key bindings.
#[derive(Clone, Debug, Default)]
pub(super) struct KeyTrie {
    binding: Option<CompiledBinding>,
    children: IndexMap<KeyStroke, KeyTrie>,
}

impl KeyTrie {
    pub(super) fn takes_first(&self, key: KeyStroke) -> bool {
        self.children.contains_key(&key)
    }

    pub(super) fn visit_bindings<'a>(
        &'a self,
        keys: &mut Vec<KeyStroke>,
        visit: &mut impl FnMut(&[KeyStroke], &'a CompiledBinding),
    ) {
        if let Some(binding) = &self.binding {
            visit(keys, binding);
        }
        for (key, child) in &self.children {
            keys.push(*key);
            child.visit_bindings(keys, visit);
            keys.pop();
        }
    }

    fn inherit(&mut self, shared: &Self) {
        if self.binding.is_some() {
            return;
        }
        for (key, child) in &shared.children {
            match self.children.entry(*key) {
                Entry::Occupied(mut entry) => {
                    entry.get_mut().inherit(child);
                }
                Entry::Vacant(entry) => {
                    entry.insert(child.clone());
                }
            }
        }
    }

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

/// Compiled action stored at a terminal trie node.
#[derive(Clone, Debug)]
pub(super) struct CompiledBinding {
    pub(super) action: KeymapAction,
    pub(super) desc: Option<String>,
}

/// Next-key candidate returned while a key sequence is pending.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct KeymapCandidate {
    pub(crate) key: KeyStroke,
    pub(crate) action: Option<KeymapAction>,
    pub(crate) desc: Option<String>,
}

/// Runtime binding whose action is not limited to static command ids.
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

/// Result of searching a trie for a key sequence.
pub(super) enum TrieSearch {
    Matched(CompiledBinding),
    Pending(Vec<KeymapCandidate>),
    NotFound,
}
