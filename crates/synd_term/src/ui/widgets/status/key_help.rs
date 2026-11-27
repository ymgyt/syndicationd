use indexmap::IndexMap;
use ratatui::text::{Line, Span};
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

pub(super) fn line(keymap: &Keymap, layers: &LayerStack, width: u16) -> Line<'static> {
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
        return fit(descriptions(hints), None, width);
    }
    let focus = layers.top();
    let close = match focus {
        Some(Layer::CategoryFilter | Layer::SearchPrompt) => Some(CommandId::DeactivateFiltering),
        Some(Layer::Subscribe) => Some(CommandId::CancelSubscribe),
        Some(Layer::UnsubscribePopup) => Some(CommandId::CancelFeedUnsubscriptionPopup),
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
        // The form's cancel hint comes last, with space reserved even on narrow terminals.
        let trailing = if focus == Some(Layer::Subscribe)
            && hints
                .first()
                .is_some_and(|hint| hint.command == Some(close))
        {
            descriptions([hints.remove(0)]).pop()
        } else {
            None
        };
        return fit(descriptions(hints), trailing, width);
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
    let entries = groups
        .into_iter()
        .map(|((_, label), keys)| format!("{}:{label}", keys.join("/")))
        .collect();
    fit(entries, None, width)
}

fn fit(mut entries: Vec<String>, trailing: Option<String>, width: u16) -> Line<'static> {
    let width = usize::from(width);
    let display_width = |s: &str| Span::raw(s).width();
    let trailing = trailing.filter(|s| display_width(s) <= width);
    let room = width.saturating_sub(trailing.as_ref().map_or(0, |s| display_width(s) + 2));
    let mut used = 0;
    entries.retain(|entry| {
        let next = used + usize::from(used > 0) * 2 + display_width(entry);
        if next > room {
            return false;
        }
        used = next;
        true
    });
    entries.extend(trailing);
    Line::raw(entries.join("  "))
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
