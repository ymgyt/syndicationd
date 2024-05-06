use std::collections::HashMap;

use anyhow::{anyhow, bail};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

mod default;

pub mod macros;

use crate::{application::event::KeyEventResult, command::Command};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeymapId {
    Global = 0,
    Login = 1,
    Tabs = 2,
    Entries = 3,
    Subscription = 4,
    Filter = 5,
    CategoryFiltering = 6,
    UnsubscribePopupSelection = 7,
}

#[derive(Debug)]
pub(crate) struct Keymap {
    #[allow(dead_code)]
    id: KeymapId,
    enable: bool,
    trie: KeyTrie,
    pending_keys: Vec<KeyEvent>,
}

impl Keymap {
    /// Construct a `Keymap`
    pub fn new(id: KeymapId, trie: KeyTrie) -> Self {
        Self {
            id,
            enable: false,
            trie,
            pending_keys: Vec::with_capacity(2),
        }
    }

    pub fn from_map(id: KeymapId, map: HashMap<KeyEvent, KeyTrie>) -> Self {
        Self::new(id, KeyTrie::Node(KeyTrieNode { map }))
    }

    fn search(&mut self, event: &KeyEvent) -> Option<Command> {
        let first = self.pending_keys.first().unwrap_or(event);
        let trie = match self.trie.search(&[*first]) {
            Some(KeyTrie::Command(cmd)) => return Some(cmd),
            Some(KeyTrie::Node(node)) => KeyTrie::Node(node),
            None => return None,
        };

        self.pending_keys.push(*event);
        match trie.search(&self.pending_keys[1..]) {
            Some(KeyTrie::Command(cmd)) => {
                self.pending_keys.drain(..);
                Some(cmd)
            }
            Some(KeyTrie::Node(_)) => None,
            _ => {
                self.pending_keys.drain(..);
                None
            }
        }
    }
}

pub(crate) struct KeymapsConfig {
    pub login: KeyTrie,
    pub tabs: KeyTrie,
    pub entries: KeyTrie,
    pub subscription: KeyTrie,
    pub filter: KeyTrie,
    pub unsubscribe_popup: KeyTrie,
    pub global: KeyTrie,
}

impl Default for KeymapsConfig {
    fn default() -> Self {
        default::default()
    }
}

#[derive(Debug)]
pub(crate) struct Keymaps {
    keymaps: Vec<Keymap>,
}

impl Keymaps {
    pub fn new(config: KeymapsConfig) -> Self {
        // order is matter
        let keymaps = vec![
            Keymap::new(KeymapId::Global, config.global),
            Keymap::new(KeymapId::Login, config.login),
            Keymap::new(KeymapId::Tabs, config.tabs),
            Keymap::new(KeymapId::Entries, config.entries),
            Keymap::new(KeymapId::Subscription, config.subscription),
            Keymap::new(KeymapId::Filter, config.filter),
            Keymap::new(KeymapId::CategoryFiltering, KeyTrie::default()),
            Keymap::new(
                KeymapId::UnsubscribePopupSelection,
                config.unsubscribe_popup,
            ),
        ];

        Self { keymaps }
    }

    pub fn enable(&mut self, id: KeymapId) {
        self.keymaps[id as usize].enable = true;
    }

    pub fn disable(&mut self, id: KeymapId) {
        self.keymaps[id as usize].enable = false;
    }

    pub fn toggle(&mut self, id: KeymapId) {
        let enable = self.keymaps[id as usize].enable;
        self.keymaps[id as usize].enable = !enable;
    }

    pub fn update(&mut self, id: KeymapId, keymap: Keymap) {
        let mut keymap = keymap;
        keymap.enable = true;
        self.keymaps[id as usize] = keymap;
    }

    pub fn search(&mut self, event: &KeyEvent) -> KeyEventResult {
        for keymap in self.keymaps.iter_mut().rev().filter(|k| k.enable) {
            if let Some(cmd) = keymap.search(event) {
                return KeyEventResult::Consumed(Some(cmd));
            }
        }
        KeyEventResult::Ignored
    }
}

impl Default for Keymaps {
    fn default() -> Self {
        Self::new(KeymapsConfig::default())
    }
}

#[derive(Clone, Debug)]
pub(crate) enum KeyTrie {
    Command(Command),
    Node(KeyTrieNode),
}

impl KeyTrie {
    pub fn search(&self, keys: &[KeyEvent]) -> Option<KeyTrie> {
        let mut trie = self;
        for key in keys {
            trie = match trie {
                KeyTrie::Command(_) => return Some(trie.clone()),
                KeyTrie::Node(trie) => trie.map.get(key)?,
            }
        }
        Some(trie.clone())
    }
}

impl Default for KeyTrie {
    fn default() -> Self {
        KeyTrie::Node(KeyTrieNode {
            map: HashMap::new(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct KeyTrieNode {
    map: HashMap<KeyEvent, KeyTrie>,
}

fn parse(s: &str) -> anyhow::Result<KeyEvent> {
    let mut tokens: Vec<_> = s.split('-').collect();
    let code = match tokens.pop().ok_or_else(|| anyhow!("no token"))? {
        "enter" => KeyCode::Enter,
        "tab" => KeyCode::Tab,
        "backtab" => KeyCode::BackTab,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "esc" => KeyCode::Esc,
        single if single.chars().count() == 1 => KeyCode::Char(single.chars().next().unwrap()),
        undefined => bail!("`{undefined}` is not implemented yet"),
    };

    let mut modifiers = KeyModifiers::NONE;
    for token in tokens {
        let modifier = match token {
            "C" => KeyModifiers::CONTROL,
            undefined => bail!("`{undefined}` modifier is not implemented yet"),
        };
        modifiers.insert(modifier);
    }
    Ok(KeyEvent::new(code, modifiers))
}
