use core::assert_matches;

use crate::command::FilterTarget;
use synd_feed::types::Category;

use super::{default::default_keymap_config, *};

#[test]
fn command_registry_uses_only_timeline_refresh_names() {
    let registry = CommandRegistry;

    assert_eq!(
        registry.command_id("timeline.refresh").unwrap(),
        CommandId::RefreshTimeline
    );
    assert_eq!(
        registry.command_id(":refresh-timeline").unwrap(),
        CommandId::RefreshTimeline
    );
    assert_matches!(
        registry.command_id("entries.reload"),
        Err(KeymapError::UnknownCommand(_))
    );
    assert_matches!(
        registry.command_id("reload_entries"),
        Err(KeymapError::UnknownCommand(_))
    );
    assert_matches!(
        registry.command_id(":reload-entries"),
        Err(KeymapError::UnknownCommand(_))
    );
}

#[test]
fn default_keymap_matches_single_key_binding() {
    let mut keymap = Keymap::default_keymaps();
    let layers = LayerStack::from([Layer::App, Layer::Global, Layer::List, Layer::Entries]);

    let result = keymap.resolve(&layers, key("j"));

    assert_matches!(
        result,
        KeymapResult::Matched {
            layer: Layer::List,
            action: KeymapAction::Command(CommandId::ListNext),
        }
    );
}

#[test]
fn covered_list_consumes_its_bindings_without_running_them() {
    let mut config = default_keymap_config();
    config
        .add(Layer::Feeds, ["q"], CommandId::OpenFeed, None)
        .unwrap();
    let mut keymap = Keymap::new(CompiledKeymaps::compile(config, &CommandRegistry).unwrap());
    let mut layers = LayerStack::from([Layer::Global, Layer::List, Layer::Feeds]);
    layers.cover(&[Layer::List, Layer::Feeds]);
    layers.extend(&[Layer::UnsubscribePopup]);

    let hints = keymap.hints(&layers);
    assert!(hints.iter().any(|hint| hint.keys == "Ent"));
    assert!(
        !hints
            .iter()
            .any(|hint| matches!(hint.keys.as_str(), "q" | "j" | "g e"))
    );
    assert_matches!(
        matched_action(&keymap.resolve(&layers, key("q"))),
        Some(KeymapAction::Command(CommandId::Nop))
    );
    assert_matches!(
        keymap.resolve(&layers, key("g")),
        KeymapResult::Pending { .. }
    );
    assert_matches!(
        matched_action(&keymap.resolve(&layers, key("e"))),
        Some(KeymapAction::Command(CommandId::Nop))
    );
}

#[test]
fn view_override_keeps_other_list_bindings_with_the_same_prefix() {
    let mut user = KeymapConfig::new();
    user.add(Layer::Entries, ["g", "g"], CommandId::OpenEntry, None)
        .unwrap();
    let mut keymap = Keymap::new(CompiledKeymaps::default_with_user_config(user).unwrap());
    let layers = LayerStack::from([Layer::List, Layer::Entries]);
    for (last_key, expected) in [("g", CommandId::OpenEntry), ("e", CommandId::ListLast)] {
        assert_matches!(
            keymap.resolve(&layers, key("g")),
            KeymapResult::Pending { .. }
        );
        let hints = keymap.hints(&layers);
        assert_eq!(hints.len(), 2);
        assert!(
            hints
                .iter()
                .any(|hint| hint.keys == "g" && hint.command == Some(CommandId::OpenEntry))
        );
        assert!(
            hints
                .iter()
                .any(|hint| hint.keys == "e" && hint.description == Some("Last"))
        );
        assert_eq!(
            matched_action(&keymap.resolve(&layers, key(last_key))),
            Some(&expected.into())
        );
    }
}

#[test]
fn user_override_can_disable_default_binding_with_no_op() {
    let mut config = default_keymap_config();
    let mut user = KeymapConfig::new();
    user.add(Layer::Entries, ["j"], CommandId::Nop, None)
        .unwrap();
    config.merge(user);

    let mut keymap = Keymap::new(CompiledKeymaps::compile(config, &CommandRegistry).unwrap());
    let layers = LayerStack::from([Layer::List, Layer::Entries]);

    let result = keymap.resolve(&layers, key("j"));

    assert_matches!(
        matched_action(&result),
        Some(KeymapAction::Command(CommandId::Nop))
    );
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
    let layers = LayerStack::from([Layer::List, Layer::Entries]);

    let result = keymap.resolve(&layers, key("up"));

    assert_matches!(
        matched_action(&result),
        Some(KeymapAction::Command(CommandId::Nop))
    );

    let result = keymap.resolve(&layers, key("g"));
    assert_matches!(result, KeymapResult::Pending { .. });

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

    assert_matches!(
        CompiledKeymaps::default_with_user_config(user),
        Err(KeymapError::CommandNotAllowed {
            layer: Layer::Feeds,
            command: CommandId::MoveEntryNext,
        })
    );
}

#[test]
fn app_layer_can_override_global_layer() {
    let mut keymap = Keymap::default_keymaps();
    let layers = LayerStack::from([Layer::Global, Layer::App]);

    let result = keymap.resolve(&layers, key("C-c"));

    assert_matches!(
        matched_action(&result),
        Some(KeymapAction::Command(CommandId::Quit))
    );
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
                target: FilterTarget::Feeds,
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
            target: FilterTarget::Feeds,
            category: rust,
        }))
    );
}

#[test]
fn shifted_letter_from_terminal_matches_s_notation_binding() {
    let mut keymap = Keymap::default_keymaps();
    let layers = LayerStack::from([Layer::App, Layer::Global, Layer::List, Layer::Entries]);

    // Terminals report Shift+t as an uppercase char + SHIFT
    let event = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('T'),
        crossterm::event::KeyModifiers::SHIFT,
    );
    let result = keymap.resolve(&layers, event);
    assert_eq!(
        matched_action(&result),
        Some(&KeymapAction::Command(CommandId::RotateTheme))
    );

    // The kitty keyboard protocol reports the base key: lowercase + SHIFT
    let event = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('r'),
        crossterm::event::KeyModifiers::SHIFT,
    );
    let result = keymap.resolve(&layers, event);
    assert_eq!(
        matched_action(&result),
        Some(&KeymapAction::Command(CommandId::ForceRedraw))
    );
}

fn matched_action(result: &KeymapResult) -> Option<&KeymapAction> {
    match result {
        KeymapResult::Matched { action, .. } => Some(action),
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
