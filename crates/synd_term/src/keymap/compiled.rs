use super::{
    CommandId, CommandRegistry, Keymap, KeymapAction, KeymapConfig, KeymapError, Layer,
    default::default_keymap_config,
};
#[derive(Clone, Debug)]
pub struct CompiledKeymaps(pub(crate) Keymap);
impl CompiledKeymaps {
    pub(crate) fn compile(
        config: KeymapConfig,
        registry: &CommandRegistry,
    ) -> Result<Self, KeymapError> {
        let config = config.try_map_commands(|layer, command: CommandId| {
            registry.validate_binding(*layer, command)?;
            Ok::<_, KeymapError>(KeymapAction::from(command))
        })?;
        let mut map = Keymap::compile(config)?;
        map.set_layer(Layer::SearchPrompt, super::search_prompt());
        map.set_layer(Layer::TextInput, super::runtime::line_editor());
        for layer in [Layer::Entries, Layer::Feeds, Layer::GhNotifications] {
            map.inherit_shared_prefixes(layer, Layer::List);
        }
        Ok(Self(map))
    }
    pub fn default_keymaps() -> Self {
        Self::compile(default_keymap_config(), &CommandRegistry).expect("valid defaults")
    }
    pub fn default_with_user_config(user: KeymapConfig) -> Result<Self, KeymapError> {
        Self::compile(
            default_keymap_config().with_overrides(user)?,
            &CommandRegistry,
        )
    }
}
impl Default for CompiledKeymaps {
    fn default() -> Self {
        Self::default_keymaps()
    }
}
