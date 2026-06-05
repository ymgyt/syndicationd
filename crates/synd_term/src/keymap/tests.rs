use crate::{
    application::Direction,
    command::{Command, FeedsCommand, ShellCommand},
    ui::widgets::filter::FilterLane,
};
use synd_feed::types::Category;

use super::{default::default_keymap_config, *};

#[test]
fn command_registry_resolves_canonical_alias_and_typable_names() {
    let registry = CommandRegistry;

    assert_eq!(
        registry.command_id("entries.reload").unwrap(),
        CommandId::ReloadEntries
    );
    assert_eq!(
        registry.command_id("reload_entries").unwrap(),
        CommandId::ReloadEntries
    );
    assert_eq!(
        registry.command_id(":reload-entries").unwrap(),
        CommandId::ReloadEntries
    );
}

#[test]
fn default_keymap_matches_single_key_binding() {
    let mut keymap = Keymap::default_keymaps();
    let layers = LayerStack::from([Layer::App, Layer::Global, Layer::Entries]);

    let result = keymap.resolve(&layers, key("j"));

    assert_eq!(
        matched_action(&result),
        Some(&KeymapAction::Command(CommandId::MoveEntryNext))
    );
    assert!(matches!(
        result_to_command(&result),
        Some(Command::Feeds(FeedsCommand::MoveEntry(Direction::Down)))
    ));
}

#[test]
fn resolver_keeps_sequence_on_highest_priority_first_key_layer() {
    let mut keymap = Keymap::default_keymaps();
    let layers = LayerStack::from([Layer::Feeds, Layer::Entries]);

    assert!(matches!(
        keymap.resolve(&layers, key("g")),
        KeymapResult::Pending { .. }
    ));
    let result = keymap.resolve(&layers, key("e"));

    assert_eq!(
        matched_action(&result),
        Some(&KeymapAction::Command(CommandId::MoveEntryLast))
    );
}

#[test]
fn user_override_can_disable_default_binding_with_no_op() {
    let mut config = default_keymap_config();
    let mut user = KeymapConfig::new();
    user.add(Layer::Entries, ["j"], CommandId::Nop, None)
        .unwrap();
    config.merge(user);

    let mut keymap = Keymap::new(CompiledKeymaps::compile(config, &CommandRegistry).unwrap());
    let layers = LayerStack::from([Layer::Entries]);

    let result = keymap.resolve(&layers, key("j"));

    assert!(matches!(result_to_command(&result), Some(Command::Nop)));
}

#[test]
fn user_facing_config_deserializes_and_merges_with_default_keymaps() {
    let user: KeymapConfig = toml::from_str(
        r#"
[entries]
keymap = [
  { on = "j", command = "entries.next", desc = "Next entry" },
  { on = ["g", "g"], command = "entries.first", desc = "Go to first entry" },
  { on = "up", command = "no_op" },
]
"#,
    )
    .unwrap();
    let mut keymap = Keymap::new(CompiledKeymaps::default_with_user_config(user).unwrap());
    let layers = LayerStack::from([Layer::Entries]);

    let result = keymap.resolve(&layers, key("up"));

    assert!(matches!(result_to_command(&result), Some(Command::Nop)));

    let result = keymap.resolve(&layers, key("g"));
    assert!(matches!(result, KeymapResult::Pending { .. }));

    let result = keymap.resolve(&layers, key("g"));
    assert_eq!(
        matched_action(&result),
        Some(&KeymapAction::Command(CommandId::MoveEntryFirst))
    );
}

#[test]
fn user_facing_config_rejects_command_in_wrong_layer() {
    let user: KeymapConfig = toml::from_str(
        r#"
[feeds]
keymap = [
  { on = "j", command = "entries.next" },
]
"#,
    )
    .unwrap();

    assert!(matches!(
        CompiledKeymaps::default_with_user_config(user),
        Err(KeymapError::CommandNotAllowed {
            layer: Layer::Feeds,
            command: CommandId::MoveEntryNext,
        })
    ));
}

#[test]
fn app_layer_can_override_global_layer() {
    let mut keymap = Keymap::default_keymaps();
    let layers = LayerStack::from([Layer::Global, Layer::App]);

    let result = keymap.resolve(&layers, key("C-c"));

    assert!(matches!(
        result_to_command(&result),
        Some(Command::Shell(ShellCommand::Quit))
    ));
}

#[test]
fn search_prompt_layer_turns_text_input_into_actions() {
    let mut keymap = Keymap::default_keymaps();
    keymap.set_layer_keymap(LayerKeymap::search_prompt());
    let layers = LayerStack::from([Layer::App, Layer::Filter, Layer::SearchPrompt]);

    let result = keymap.resolve(&layers, key("a"));

    assert_eq!(
        matched_action(&result),
        Some(&KeymapAction::Prompt(PromptAction::InsertChar('a')))
    );

    let result = keymap.resolve(&layers, key("esc"));

    assert_eq!(
        matched_action(&result),
        Some(&KeymapAction::Command(CommandId::DeactivateFiltering))
    );

    let result = keymap.resolve(&layers, key("C-c"));

    assert_eq!(
        matched_action(&result),
        Some(&KeymapAction::Command(CommandId::Quit))
    );
}

#[test]
fn dynamic_category_filter_layer_resolves_runtime_category_actions() {
    let rust = Category::new("rust").unwrap();
    let mut category = LayerKeymap::builder(Layer::CategoryFilter);
    category
        .bind_key(
            KeyStroke::from_char('r'),
            KeymapAction::Filter(FilterAction::ToggleCategory {
                lane: FilterLane::Feed,
                category: rust.clone(),
            }),
            Some("Toggle rust category".to_owned()),
        )
        .unwrap();

    let mut keymap = Keymap::default_keymaps();
    keymap.set_layer_keymap(category.build().unwrap());
    let layers = LayerStack::from([Layer::Filter, Layer::CategoryFilter]);

    let result = keymap.resolve(&layers, key("r"));

    assert_eq!(
        matched_action(&result),
        Some(&KeymapAction::Filter(FilterAction::ToggleCategory {
            lane: FilterLane::Feed,
            category: rust,
        }))
    );
}

fn result_to_command(result: &KeymapResult) -> Option<Command> {
    match result {
        KeymapResult::Matched(action) => Some(action.build_command()),
        KeymapResult::Pending { .. } | KeymapResult::NotFound | KeymapResult::Cancelled { .. } => {
            None
        }
    }
}

fn matched_action(result: &KeymapResult) -> Option<&KeymapAction> {
    match result {
        KeymapResult::Matched(action) => Some(action),
        KeymapResult::Pending { .. } | KeymapResult::NotFound | KeymapResult::Cancelled { .. } => {
            None
        }
    }
}

fn key(notation: &str) -> crossterm::event::KeyEvent {
    notation
        .parse::<KeyStroke>()
        .expect("valid key notation")
        .into()
}
