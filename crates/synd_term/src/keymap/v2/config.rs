use std::collections::HashMap;

use serde::Deserialize;

use super::{CommandId, KeyBinding, KeySequence, KeymapError, Layer};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct KeymapConfig {
    pub(super) bindings: HashMap<Layer, Vec<KeyBinding>>,
}

impl KeymapConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn add<const N: usize>(
        &mut self,
        layer: Layer,
        keys: [&str; N],
        command: CommandId,
        desc: Option<&str>,
    ) -> Result<(), KeymapError> {
        let on = KeySequence::parse(keys)?;
        self.bindings
            .entry(layer)
            .or_default()
            .push(KeyBinding::new(on, command, desc.map(str::to_owned)));
        Ok(())
    }

    pub(crate) fn merge(&mut self, other: Self) {
        for (layer, bindings) in other.bindings {
            let dst = self.bindings.entry(layer).or_default();
            for binding in bindings {
                match dst.iter().position(|b| b.on == binding.on) {
                    Some(idx) => dst[idx] = binding,
                    None => dst.push(binding),
                }
            }
        }
    }
}

impl<'de> Deserialize<'de> for KeymapConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let layers = HashMap::<Layer, LayerConfig>::deserialize(deserializer)?;
        let mut bindings = HashMap::with_capacity(layers.len());
        for (layer, layer_config) in layers {
            bindings.insert(
                layer,
                layer_config.keymap.into_iter().map(Into::into).collect(),
            );
        }
        Ok(Self { bindings })
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LayerConfig {
    #[serde(default)]
    keymap: Vec<KeyBindingEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct KeyBindingEntry {
    #[serde(deserialize_with = "deserialize_key_sequence")]
    on: KeySequence,
    command: CommandId,
    desc: Option<String>,
}

impl From<KeyBindingEntry> for KeyBinding {
    fn from(entry: KeyBindingEntry) -> Self {
        Self::new(entry.on, entry.command, entry.desc)
    }
}

impl<'de> Deserialize<'de> for KeyBinding {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        KeyBindingEntry::deserialize(deserializer).map(Into::into)
    }
}

fn deserialize_key_sequence<'de, D>(deserializer: D) -> Result<KeySequence, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = RawKeySequence::deserialize(deserializer)?;
    let keys = match raw {
        RawKeySequence::One(key) => vec![key],
        RawKeySequence::Many(keys) => keys,
    };
    let keys = keys
        .into_iter()
        .map(|key| key.parse())
        .collect::<Result<Vec<_>, _>>()
        .map_err(serde::de::Error::custom)?;
    KeySequence::new(keys).map_err(serde::de::Error::custom)
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawKeySequence {
    One(String),
    Many(Vec<String>),
}
