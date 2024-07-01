use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Widget},
};

use crate::{
    client::github::{FetchNotificationInclude, FetchNotificationParticipating},
    types::github::RepoVisibility,
    ui::{
        components::gh_notifications::{
            GhNotificationFilterOptions, GhNotificationFilterOptionsState,
            GhNotificationFilterUpdater,
        },
        icon, Context,
    },
};

pub(super) struct FilterPopup {
    pub(super) is_active: bool,
    options: GhNotificationFilterOptions,
    pending_options: Option<GhNotificationFilterOptions>,
}

impl FilterPopup {
    pub(super) fn new(options: GhNotificationFilterOptions) -> Self {
        Self {
            is_active: false,
            options,
            pending_options: None,
        }
    }

    pub(super) fn applied_options(&self) -> &GhNotificationFilterOptions {
        &self.options
    }

    pub(super) fn commit(&mut self) -> GhNotificationFilterOptionsState {
        if let Some(options) = self.pending_options.take() {
            let org = std::mem::replace(&mut self.options, options);
            if org != self.options {
                return GhNotificationFilterOptionsState::Changed(self.options.clone());
            }
        }
        GhNotificationFilterOptionsState::Unchanged
    }

    pub(super) fn update_options(&mut self, new: &GhNotificationFilterUpdater) {
        let mut pending = self
            .pending_options
            .take()
            .unwrap_or_else(|| self.options.clone());

        if new.toggle_include {
            pending.include = match pending.include {
                FetchNotificationInclude::OnlyUnread => FetchNotificationInclude::All,
                FetchNotificationInclude::All => FetchNotificationInclude::OnlyUnread,
            };
        }
        if new.toggle_participating {
            pending.participating = match pending.participating {
                FetchNotificationParticipating::OnlyParticipating => {
                    FetchNotificationParticipating::All
                }
                FetchNotificationParticipating::All => {
                    FetchNotificationParticipating::OnlyParticipating
                }
            };
        }
        if new.toggle_visilibty_all {
            pending.visibility = None;
        }
        if new.toggle_visilibty_public {
            pending.visibility = Some(RepoVisibility::Public);
        }
        if new.toggle_visilibty_private {
            pending.visibility = Some(RepoVisibility::Private);
        }

        self.pending_options = Some(pending);
    }
}

impl FilterPopup {
    pub(super) fn render(&self, area: Rect, buf: &mut Buffer, cx: &Context<'_>) {
        let area = {
            let block = Block::new()
                .title_top("Filter")
                .title_alignment(Alignment::Center)
                .title_style(Style::new().add_modifier(Modifier::BOLD))
                .padding(Padding {
                    left: 1,
                    right: 1,
                    top: 1,
                    bottom: 1,
                })
                .borders(Borders::ALL)
                .style(cx.theme.base);
            let inner_area = block.inner(area);
            block.render(area, buf);
            inner_area
        };

        let vertical = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ]);
        let [status_area, participating_area, visibility_area] = vertical.areas(area);
        let options = self.pending_options.as_ref().unwrap_or(&self.options);

        // Render status
        {
            let mut spans = vec![
                Span::from(concat!(icon!(unread), " Status")).bold(),
                Span::from("         "),
            ];
            let mut unread = Span::from("Unread(u)").underlined();
            if options.include == FetchNotificationInclude::All {
                unread = unread.dim().not_underlined();
            }
            spans.push(unread);

            Line::from(spans).render(status_area, buf);
        }

        // Render participating
        {
            let mut spans = vec![
                Span::from(concat!(icon!(chat), " Participating")).bold(),
                Span::from("  "),
            ];
            let mut participating = Span::from("Participating(p)").underlined();
            if options.participating == FetchNotificationParticipating::All {
                participating = participating.dim().not_underlined();
            }
            spans.push(participating);
            Line::from(spans).render(participating_area, buf);
        }

        // Render repository visibility
        {
            let mut spans = vec![
                Span::from(concat!(icon!(repository), " Repository")).bold(),
                Span::from("     "),
            ];
            let mut all = Span::from("All(va)").dim();
            let mut public = Span::from("Public(vb)").dim();
            let mut private = Span::from("Private(vc)").dim();
            match options.visibility {
                Some(RepoVisibility::Public) => public = public.not_dim().underlined(),
                Some(RepoVisibility::Private) => private = private.not_dim().underlined(),
                None => all = all.not_dim().underlined(),
            };
            spans.extend([all, Span::from("  "), public, Span::from("  "), private]);
            Line::from(spans).render(visibility_area, buf);
        }
    }
}
