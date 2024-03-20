use std::collections::HashMap;

use anyhow::{anyhow, bail};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

mod default;

pub mod macros;

use crate::command::Command;

#[derive(Debug)]
pub struct Keymaps {
    pub login: KeyTrie,
    pub tabs: KeyTrie,
    pub entries: KeyTrie,
    pub subscription: KeyTrie,
    pub global: KeyTrie,
}

impl Default for Keymaps {
    fn default() -> Self {
        default::default()
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
