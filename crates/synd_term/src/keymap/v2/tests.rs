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
    let keymaps = KeymapRuntime::default_keymaps();
    let layers = LayerStack::from([Layer::App, Layer::Global, Layer::Entries]);
    let mut resolver = KeyResolver::new();

    let result = resolver.resolve(&keymaps, &layers, "j".parse().unwrap());

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
    let keymaps = KeymapRuntime::default_keymaps();
    let layers = LayerStack::from([Layer::Feeds, Layer::Entries]);
    let mut resolver = KeyResolver::new();

    assert!(matches!(
        resolver.resolve(&keymaps, &layers, "g".parse().unwrap()),
        KeymapResult::Pending { .. }
    ));
    let result = resolver.resolve(&keymaps, &layers, "e".parse().unwrap());

    assert_eq!(
        matched_action(&result),
        Some(&KeymapAction::Command(CommandId::MoveEntryLast))
    );
}

#[test]
fn user_override_can_disable_default_binding_with_no_op() {
    let mut config = default_keymap_config();
    let mut user = KeymapConfig::new();
    user.add(Layer::Entries, ["j"], CommandId::NoOp, None)
        .unwrap();
    config.merge(user);

    let keymaps = KeymapRuntime::new(CompiledKeymaps::compile(config, &CommandRegistry).unwrap());
    let layers = LayerStack::from([Layer::Entries]);
    let mut resolver = KeyResolver::new();

    let result = resolver.resolve(&keymaps, &layers, "j".parse().unwrap());

    assert!(matches!(result, KeymapResult::NoOp));
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
    let keymaps = KeymapRuntime::new(CompiledKeymaps::default_with_user_config(user).unwrap());
    let layers = LayerStack::from([Layer::Entries]);
    let mut resolver = KeyResolver::new();

    let result = resolver.resolve(&keymaps, &layers, "up".parse().unwrap());

    assert!(matches!(result, KeymapResult::NoOp));

    let result = resolver.resolve(&keymaps, &layers, "g".parse().unwrap());
    assert!(matches!(result, KeymapResult::Pending { .. }));

    let result = resolver.resolve(&keymaps, &layers, "g".parse().unwrap());
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
    let keymaps = KeymapRuntime::default_keymaps();
    let layers = LayerStack::from([Layer::Global, Layer::App]);
    let mut resolver = KeyResolver::new();

    let result = resolver.resolve(&keymaps, &layers, "C-c".parse().unwrap());

    assert!(matches!(
        result_to_command(&result),
        Some(Command::Shell(ShellCommand::Quit))
    ));
}

#[test]
fn search_prompt_layer_turns_text_input_into_actions() {
    let mut keymaps = KeymapRuntime::default_keymaps();
    keymaps.set_layer_keymap(LayerKeymap::search_prompt());
    let layers = LayerStack::from([Layer::App, Layer::Filter, Layer::SearchPrompt]);
    let mut resolver = KeyResolver::new();

    let result = resolver.resolve(&keymaps, &layers, "a".parse().unwrap());

    assert_eq!(
        matched_action(&result),
        Some(&KeymapAction::Prompt(PromptAction::InsertChar('a')))
    );

    let result = resolver.resolve(&keymaps, &layers, "esc".parse().unwrap());

    assert_eq!(
        matched_action(&result),
        Some(&KeymapAction::Command(CommandId::DeactivateFiltering))
    );

    let result = resolver.resolve(&keymaps, &layers, "C-c".parse().unwrap());

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

    let mut keymaps = KeymapRuntime::default_keymaps();
    keymaps.set_layer_keymap(category.build().unwrap());
    let layers = LayerStack::from([Layer::Filter, Layer::CategoryFilter]);
    let mut resolver = KeyResolver::new();

    let result = resolver.resolve(&keymaps, &layers, "r".parse().unwrap());

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
        KeymapResult::Matched(action) => action.build_command(),
        KeymapResult::NoOp => None,
        KeymapResult::Pending { .. } | KeymapResult::NotFound | KeymapResult::Cancelled { .. } => {
            None
        }
    }
}

fn matched_action(result: &KeymapResult) -> Option<&KeymapAction> {
    match result {
        KeymapResult::Matched(action) => Some(action),
        KeymapResult::NoOp
        | KeymapResult::Pending { .. }
        | KeymapResult::NotFound
        | KeymapResult::Cancelled { .. } => None,
    }
}
