use indexmap::IndexMap;
use std::collections::HashSet;

use crate::{
    keymap::{CommandId, Keymap, KeymapAction, Layer, LayerStack},
    ui::icon,
};

/// Each row selects one reachable binding. Commands on the same row perform
/// the same action in different views. The two movement rows share an icon.
const MAIN_ACTIONS: &[(&[CommandId], &str)] = &[
    (&[CommandId::MoveTabNext, CommandId::MoveTabPrev], "󰹳"),
    (
        &[
            CommandId::ListNext,
            CommandId::MoveEntryNext,
            CommandId::MoveSubscribedFeedNext,
            CommandId::MoveGhNotificationNext,
            CommandId::MoveAuthenticationProviderNext,
        ],
        "󰹹",
    ),
    (
        &[
            CommandId::ListPrev,
            CommandId::MoveEntryPrev,
            CommandId::MoveSubscribedFeedPrev,
            CommandId::MoveGhNotificationPrev,
            CommandId::MoveAuthenticationProviderPrev,
        ],
        "󰹹",
    ),
    (
        &[
            CommandId::OpenEntry,
            CommandId::OpenFeed,
            CommandId::OpenGhNotification,
            CommandId::Authenticate,
        ],
        icon!(open),
    ),
    (
        &[CommandId::ReadEntry, CommandId::BrowseEntry],
        icon!(browse),
    ),
    (&[CommandId::PromptFeedSubscription], "Subscribe"),
    (&[CommandId::PromptFeedEdition], ""),
    (&[CommandId::PromptFeedUnsubscription], "󰼡"),
    (&[CommandId::MarkGhNotificationAsDone], icon!(check)),
    (&[CommandId::OpenGhNotificationFilter], icon!(filter)),
    (&[CommandId::ActivateSearchFiltering], icon!(search)),
    (&[CommandId::Quit], ""),
];

pub(super) fn entries(keymap: &Keymap, layers: &LayerStack) -> Vec<String> {
    let mut hints = keymap
        .hints(layers)
        .into_iter()
        .filter_map(|hint| {
            let command = match hint.command {
                KeymapAction::Command(CommandId::Nop) => return None,
                KeymapAction::Command(id) => Some(*id),
                _ => None,
            };
            Some(KeyHint {
                layer: hint.layer,
                keys: hint.keys,
                command,
                description: hint.description,
            })
        })
        .collect::<Vec<_>>();
    if !keymap.pending_keys().is_empty() {
        return descriptions(hints);
    }
    let focus = layers.top();
    let close = match focus {
        Some(Layer::CategoryFilter | Layer::SearchPrompt) => Some(CommandId::DeactivateFiltering),
        Some(Layer::UnsubscribePopup) => Some(CommandId::CancelFeedUnsubscriptionPopup),
        Some(Layer::Subscribe) => Some(CommandId::CancelSubscribe),
        Some(Layer::GhNotificationFilterPopup) => Some(CommandId::CloseGhNotificationFilter),
        Some(Layer::Reader) => Some(CommandId::CloseReader),
        _ => None,
    };
    if let Some(close) = close {
        let mut commands = HashSet::new();
        hints.retain(|hint| {
            (Some(hint.layer) == focus || hint.command == Some(close))
                && hint.command.is_none_or(|id| commands.insert(id))
        });
        hints.sort_by_key(|hint| hint.command != Some(close));
        return descriptions(hints);
    }

    let mut groups = IndexMap::<_, Vec<&str>>::new();
    for &(commands, default_label) in MAIN_ACTIONS {
        if let Some(hint) = hints
            .iter()
            .find(|hint| hint.command.is_some_and(|cmd| commands.contains(&cmd)))
        {
            let label = hint.description.unwrap_or(default_label);
            groups
                .entry((default_label, label))
                .or_default()
                .push(&hint.keys);
        }
    }
    groups
        .into_iter()
        .map(|((_, label), keys)| format!("{}:{label}", keys.join("/")))
        .collect()
}

fn descriptions<'a>(hints: impl IntoIterator<Item = KeyHint<'a>>) -> Vec<String> {
    let mut groups = IndexMap::<_, Vec<String>>::new();
    for hint in hints {
        if let Some(label) = hint
            .description
            .or_else(|| hint.command.map(CommandId::as_str))
        {
            groups.entry(label).or_default().push(hint.keys);
        }
    }
    groups
        .into_iter()
        .map(|(label, keys)| format!("{}:{label}", keys.join("/")))
        .collect()
}

struct KeyHint<'a> {
    layer: Layer,
    keys: String,
    command: Option<CommandId>,
    description: Option<&'a str>,
}
