//! The builder's history page: a deck's saved versions, read as snapshots
//! without borrowing the builder's editable state. The house decks are the
//! Decks screen's House tab (WP3); this page drew them too once, and stood
//! over the builder when that tab was left open (the beta.6 review).
use super::press::Cx;
#[allow(clippy::wildcard_imports)] // the lobby widget vocabulary
use super::*;
use client_core::lobby::library::{Page, Snapshot, row_changes};

#[allow(clippy::too_many_lines)] // declarative library page, like the adjacent lobby screens
pub(super) fn screen(
    commands: &mut Commands,
    root: Entity,
    state: &LobbyState,
    fonts: &UiFonts,
    metrics: Metrics,
    scrolled: &Scrolled,
) {
    let lobby = &state.lobby;
    let lib = lobby.library();
    let lang = lobby.lang();
    let bar = row(commands, metrics, true);
    commands.entity(bar).insert(Node {
        width: percent(100),
        padding: UiRect::all(px(metrics.pad * 1.5)),
        flex_wrap: FlexWrap::Wrap,
        align_items: AlignItems::Center,
        column_gap: px(metrics.gap),
        row_gap: px(metrics.gap),
        ..default()
    });
    let back = button(
        commands,
        fonts,
        metrics,
        Phrase::LibraryBack.text(lang),
        Press::Library(LibraryPress::CloseLibrary),
        palette::PANEL_LIT,
        !lib.loading,
    );
    let title = heading(commands, fonts, metrics, Phrase::DeckHistory.text(lang));
    commands.entity(bar).add_children(&[back, title]);
    commands.entity(root).add_child(bar);
    let body = scroller(
        commands,
        metrics,
        List::Library,
        scrolled.get(List::Library),
    );
    commands.entity(body).insert(Node {
        width: percent(100),
        flex_grow: 1.0,
        min_height: px(0),
        overflow: Overflow::scroll_y(),
        padding: UiRect::all(px(metrics.pad * 1.5)),
        flex_direction: FlexDirection::Column,
        row_gap: px(metrics.gap),
        ..default()
    });
    commands.entity(body).insert(super::dock::Dock(7));
    commands.entity(root).add_child(body);
    let hint = note(commands, fonts, metrics, Phrase::HistoryHint.text(lang));
    commands.entity(body).add_child(hint);
    if lib.loading {
        let loading = note(commands, fonts, metrics, Phrase::LibraryLoading.text(lang));
        commands.entity(body).add_child(loading);
    }
    if let Some(error) = &lib.error {
        let error = note(commands, fonts, metrics, error);
        commands.entity(error).insert(TextColor(palette::DANGER));
        let retry = button(
            commands,
            fonts,
            metrics,
            Phrase::LibraryRetry.text(lang),
            Press::Library(LibraryPress::RetryLibrary),
            palette::PANEL_LIT,
            !lib.loading,
        );
        commands.entity(body).add_children(&[error, retry]);
    }
    if let Some(history) = &lib.history {
        let versions = row(commands, metrics, true);
        for version in
            std::iter::once(history.version).chain(history.past.iter().map(|v| v.version))
        {
            let label = format!(
                "v{version}{}",
                if version == history.version {
                    format!(" · {}", Phrase::CurrentVersion.text(lang))
                } else {
                    String::new()
                }
            );
            let selected = lib
                .preview
                .as_ref()
                .is_some_and(|(_, s)| s.version == version);
            let control = button(
                commands,
                fonts,
                metrics,
                &label,
                Press::Library(LibraryPress::PreviewVersion(version)),
                if selected {
                    palette::ACCENT
                } else {
                    palette::PANEL_LIT
                },
                !lib.loading,
            );
            commands.entity(versions).add_child(control);
        }
        commands.entity(body).add_child(versions);
        if history.past.is_empty() {
            let empty = note(commands, fonts, metrics, Phrase::NoPastVersions.text(lang));
            commands.entity(body).add_child(empty);
        }
        if let Some((_, snapshot)) = &lib.preview {
            let card = super::surface(commands, metrics);
            let title = heading(
                commands,
                fonts,
                metrics,
                &Phrase::VersionLabel.fill(lang, &[&snapshot.version.to_string()]),
            );
            commands.entity(card).add_child(title);
            let timestamp = history
                .past
                .iter()
                .find(|v| v.version == snapshot.version)
                .map_or(history.updated_at, |v| v.superseded_at);
            if let Ok(at) = time::OffsetDateTime::from_unix_timestamp(timestamp) {
                let date = note(
                    commands,
                    fonts,
                    metrics,
                    &format!(
                        "{} · {:04}-{:02}-{:02} · {:02}:{:02} UTC",
                        if snapshot.version == history.version {
                            Phrase::VersionSavedAt
                        } else {
                            Phrase::VersionSupersededAt
                        }
                        .text(lang),
                        at.year(),
                        u8::from(at.month()),
                        at.day(),
                        at.hour(),
                        at.minute()
                    ),
                );
                commands.entity(card).add_child(date);
            }
            if let Some(revision) = history.past.iter().find(|v| v.version == snapshot.version)
                && let Some(summary) = &revision.summary
            {
                let summary = note(commands, fonts, metrics, summary);
                commands.entity(card).add_child(summary);
            }
            if snapshot.version != history.version {
                let restore = button(
                    commands,
                    fonts,
                    metrics,
                    if lib.confirm_restore {
                        Phrase::ConfirmRestore
                    } else {
                        Phrase::RestoreVersion
                    }
                    .text(lang),
                    Press::Library(LibraryPress::RestoreVersion),
                    if lib.confirm_restore {
                        palette::DANGER
                    } else {
                        palette::ACCENT
                    },
                    !lib.loading,
                );
                commands.entity(card).add_child(restore);
                if lib.confirm_restore {
                    let warning = note(commands, fonts, metrics, Phrase::RestoreWarning.text(lang));
                    commands.entity(warning).insert(TextColor(palette::ACTIVE));
                    commands.entity(card).add_child(warning);
                }
            }
            let diff_title = heading(commands, fonts, metrics, Phrase::WorkingDiff.text(lang));
            commands.entity(card).add_child(diff_title);
            let base = lib.current.as_ref().unwrap_or(snapshot);
            for (label, before, after) in [
                (Phrase::LibraryMain, &base.cards, &snapshot.cards),
                (Phrase::LibrarySide, &base.sideboard, &snapshot.sideboard),
                (
                    Phrase::LibraryCommanders,
                    &base.commanders,
                    &snapshot.commanders,
                ),
            ] {
                let section = heading(commands, fonts, metrics, label.text(lang));
                commands.entity(card).add_child(section);
                let changes = row_changes(before, after);
                if changes.is_empty() {
                    let same = note(commands, fonts, metrics, Phrase::NoChanges.text(lang));
                    commands.entity(card).add_child(same);
                }
                for (name, delta) in changes {
                    let change = note(commands, fonts, metrics, &format!("{delta:+}  {name}"));
                    commands.entity(change).insert(TextColor(if delta > 0 {
                        palette::ACCENT
                    } else {
                        palette::DANGER
                    }));
                    commands.entity(card).add_child(change);
                }
            }
            contents(commands, card, fonts, metrics, lang, snapshot);
            commands.entity(body).add_child(card);
        }
    }
}

fn contents(
    commands: &mut Commands,
    parent: Entity,
    fonts: &UiFonts,
    metrics: Metrics,
    lang: Lang,
    snapshot: &Snapshot,
) {
    let count = |rows: &[String]| {
        rows.iter()
            .map(|row| {
                row.split_once(' ')
                    .and_then(|(n, _)| n.parse::<u32>().ok())
                    .unwrap_or(1)
            })
            .sum::<u32>()
    };
    let stats = note(
        commands,
        fonts,
        metrics,
        &Phrase::LibraryCounts.fill(
            lang,
            &[
                &count(&snapshot.cards).to_string(),
                &count(&snapshot.sideboard).to_string(),
            ],
        ),
    );
    commands.entity(parent).add_child(stats);
    for (label, rows) in [
        (Phrase::LibraryCommanders, &snapshot.commanders),
        (Phrase::LibraryMain, &snapshot.cards),
        (Phrase::LibrarySide, &snapshot.sideboard),
    ] {
        if rows.is_empty() {
            continue;
        }
        let title = heading(commands, fonts, metrics, label.text(lang));
        let text = note(commands, fonts, metrics, &rows.join("\n"));
        commands.entity(parent).add_children(&[title, text]);
    }
}

/// A control of the builder's history page: a deck's saved versions.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum LibraryPress {
    BrowseHistory,
    CloseLibrary,
    RetryLibrary,
    PreviewVersion(i32),
    RestoreVersion,
}

impl LibraryPress {
    /// What a click on this control does.
    pub(super) fn handle(self, cx: Cx<'_, '_, '_, '_, '_>) {
        let Cx {
            state,
            scrolled,
            mailbox,
            ..
        } = cx;
        match self {
            LibraryPress::BrowseHistory | LibraryPress::RetryLibrary => {
                scrolled.set(List::Library, 0.0);
                // The builder's page is a deck's history and nothing else:
                // the house list is the Decks screen's own tab (WP3).
                let request = if let Some(Page::History(id)) = state.lobby.library().page.clone() {
                    state.lobby.browse_deck_history(&id)
                } else {
                    state.lobby.browse_history()
                };
                dispatch(state, mailbox, request);
            }
            LibraryPress::CloseLibrary => state.lobby.close_library(),
            LibraryPress::PreviewVersion(version) => {
                if let Some(Page::History(id)) = state.lobby.library().page.clone() {
                    let request = state.lobby.preview_version(&id, version);
                    dispatch(state, mailbox, request);
                }
            }
            LibraryPress::RestoreVersion => {
                let request = state.lobby.restore_preview();
                dispatch(state, mailbox, request);
            }
        }
    }
}
