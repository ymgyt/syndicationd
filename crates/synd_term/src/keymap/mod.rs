use std::collections::HashMap;

use anyhow::{anyhow, bail};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

mod default;

pub mod macros;

use crate::command::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeymapId {
    Global = 0,
    Login = 1,
    Tabs = 2,
    Entries = 3,
    Subscription = 4,
}

#[derive(Debug)]
struct Keymap {
    #[allow(unused)]
    id: KeymapId,
    enable: bool,
    trie: KeyTrie,
    pending_keys: Vec<KeyEvent>,
}

impl Keymap {
    fn new(id: KeymapId, trie: KeyTrie) -> Self {
        Self {
            id,
            enable: false,
            trie,
            pending_keys: Vec::with_capacity(2),
        }
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

pub struct KeymapsConfig {
    pub login: KeyTrie,
    pub tabs: KeyTrie,
    pub entries: KeyTrie,
    pub subscription: KeyTrie,
    pub global: KeyTrie,
}

impl Default for KeymapsConfig {
    fn default() -> Self {
        default::default()
    }
}

#[derive(Debug)]
pub struct Keymaps {
    keymaps: Box<[Keymap; 5]>,
}

impl Keymaps {
    pub fn new(config: KeymapsConfig) -> Self {
        // order is matter
        let keymaps = [
            Keymap::new(KeymapId::Global, config.global),
            Keymap::new(KeymapId::Login, config.login),
            Keymap::new(KeymapId::Tabs, config.tabs),
            Keymap::new(KeymapId::Entries, config.entries),
            Keymap::new(KeymapId::Subscription, config.subscription),
        ];

        Self {
            keymaps: Box::new(keymaps),
        }
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

    pub fn search(&mut self, event: KeyEvent) -> Option<Command> {
        for keymap in self.keymaps.iter_mut().rev().filter(|k| k.enable) {
            if let Some(cmd) = keymap.search(&event) {
                return Some(cmd);
            }
        }
        None
    }
}

impl Default for Keymaps {
    fn default() -> Self {
        Self::new(KeymapsConfig::default())
    }
}

#[derive(Clone, Debug)]
pub enum KeyTrie {
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
