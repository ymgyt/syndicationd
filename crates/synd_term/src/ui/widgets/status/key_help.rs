use indexmap::IndexMap;
use std::collections::HashSet;

use crate::{
    keymap::{CommandId, KeyHint, Keymap, Layer, LayerStack},
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
    (&[CommandId::BrowseEntry], icon!(browse)),
    (&[CommandId::PromptFeedSubscription], icon!(feeds)),
    (&[CommandId::PromptFeedEdition], ""),
    (&[CommandId::PromptFeedUnsubscription], "󰼡"),
    (&[CommandId::MarkGhNotificationAsDone], icon!(check)),
    (&[CommandId::OpenGhNotificationFilter], icon!(filter)),
    (&[CommandId::ActivateSearchFiltering], icon!(search)),
    (&[CommandId::Quit], ""),
];

pub(super) fn entries(keymap: &Keymap, layers: &LayerStack) -> Vec<String> {
    let mut hints = keymap.hints(layers);
    if keymap.has_pending_keys() {
        return descriptions(hints);
    }
    let focus = layers.top();
    let close = match focus {
        Some(Layer::CategoryFilter | Layer::SearchPrompt) => Some(CommandId::DeactivateFiltering),
        Some(Layer::UnsubscribePopup) => Some(CommandId::CancelFeedUnsubscriptionPopup),
        Some(Layer::GhNotificationFilterPopup) => Some(CommandId::CloseGhNotificationFilter),
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
    for &(commands, icon) in MAIN_ACTIONS {
        if let Some(hint) = hints
            .iter()
            .find(|hint| hint.command.is_some_and(|cmd| commands.contains(&cmd)))
        {
            let label = hint.description.unwrap_or(icon);
            groups.entry((icon, label)).or_default().push(&hint.keys);
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
