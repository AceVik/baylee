//! A deck's history (`.claude/ux-b6/windows-b6/DESIGN.md` §C.3): one sheet
//! from both doors — a tile's ⋯ › History… on Decks and the builder's
//! History — versions left with their time and what each save changed,
//! the classified changes right (`baylee_core::deckdiff`, the classifier the
//! gateway's WG-6 counts with), compared with the version before, the deck
//! as it is now, or a second version picked from the list.
//!
//! Restore asks nothing on the shelf (the Undo toast is the safety, S-10);
//! over the builder with unsaved edits it asks the builder's one question,
//! "Discard changes?", first, because loading the restored deck replaces
//! them (Q-C3). A house deck's history is read-only (Q-C4).

use super::decks::DecksPress;
use super::orders;
use super::parts;
#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use super::*;
use crate::shellkit::controls::{self, Kit, Live, Weight};
use crate::shellkit::surfaces::{self, SheetWidth};
use crate::shellkit::{Frame as ShellFrame, px_fixed, tokens};
use baylee_core::deckdiff::{self, Change, Delta, ZoneDiff};
use baylee_core::deckrow::{self, PrintChoice};
use baylee_core::preset::Finish;
use client_core::lobby::library::{Compare, Page, Snapshot};

/// The compare bar's three choices, in their order.
const COMPARE: [Compare; 3] = [Compare::Previous, Compare::Current, Compare::Pick(None)];

/// Draws the history sheet over whatever stands under it (Decks or the
/// builder), when a deck's history is open.
#[allow(clippy::too_many_lines)] // a sheet, read top to bottom
pub(super) fn sheet(commands: &mut Commands, root: Entity, state: &LobbyState, kit: Kit) {
    let lobby = &state.lobby;
    let lang = lobby.lang();
    let lib = lobby.library();
    let Some(Page::History(id)) = &lib.page else {
        return;
    };
    let house = lobby.house_deck(id);
    let name = lobby
        .decks()
        .iter()
        .find(|d| &d.id == id)
        .map(|d| d.name.clone())
        .or_else(|| {
            lib.house
                .iter()
                .find(|d| &d.id == id)
                .map(|d| d.name.clone())
        })
        .unwrap_or_default();
    let phone = kit.m.frame == ShellFrame::Phone;
    let columns = commands
        .spawn((
            Node {
                column_gap: px_fixed(kit.m.gap * 1.5),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Start,
                width: Val::Percent(100.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let versions = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: kit.m.px(2.0),
                width: kit.m.px(if phone { 200.0 } else { 300.0 }),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let detail = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: kit.m.px(6.0),
                flex_grow: 1.0,
                flex_shrink: 1.0,
                min_width: px(0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(columns).add_children(&[versions, detail]);

    if lib.history.is_none() {
        if let Some(error) = &lib.error {
            let line = parts::line(commands, kit, error, kit.m.small, tokens::DANGER);
            let retry = controls::button(
                commands,
                kit,
                Phrase::LibraryRetry.text(lang),
                Weight::Secondary,
                Live::Yes,
                None,
                Press::Decks(DecksPress::Retry),
            );
            commands.entity(versions).add_children(&[line, retry]);
        } else {
            let skeleton = crate::shellkit::states::skeleton(commands, kit, 5);
            commands.entity(versions).add_child(skeleton);
            let skeleton = crate::shellkit::states::skeleton(commands, kit, 4);
            commands.entity(detail).add_child(skeleton);
        }
    }
    if let Some(history) = &lib.history {
        let picked = match lib.compare {
            Compare::Pick(Some(other)) => Some(other),
            _ => None,
        };
        for (walked, version) in history.versions().into_iter().enumerate() {
            let at = if version == history.version {
                Some(history.updated_at)
            } else {
                history.started(version)
            };
            let number = if version == history.version {
                Phrase::HistoryCurrentRow.fill(lang, &[&version.to_string()])
            } else {
                Phrase::HistoryRow.fill(lang, &[&version.to_string()])
            };
            let mark = if picked == Some(version) {
                "\u{25c6} "
            } else if version == history.version {
                "\u{25cf} "
            } else {
                "\u{25cb} "
            };
            // A version's time is its start; the oldest kept has none the
            // store knows (`DESIGN` §C.0), the head its own save's.
            let title = match at {
                Some(at) => format!("{mark}{number} · {}", when(at, lang)),
                None => format!("{mark}{number} · {}", Phrase::HistoryFirstSave.text(lang)),
            };
            let meta = history
                .summary(version)
                .map(str::to_string)
                .or_else(|| history.delta(version).map(|d| delta_line(&d, lang)))
                .unwrap_or_default();
            let row = surfaces::list_row(
                commands,
                kit,
                &title,
                &meta,
                &[],
                Press::Decks(DecksPress::Version(version)),
            );
            let on = lib.selected == Some(version);
            commands.entity(row).insert(BackgroundColor(if on {
                tokens::SELECTED
            } else {
                Color::NONE
            }));
            orders::item(commands, row, &orders::HISTORY, "versions", walked);
            commands.entity(versions).add_child(row);
        }
        if history.past.is_empty() {
            let none = parts::line(
                commands,
                kit,
                Phrase::NoPastVersions.text(lang),
                kit.m.small,
                tokens::MUTED,
            );
            commands.entity(versions).add_child(none);
        }
        compare_bar(commands, detail, state, kit);
        changes(commands, detail, state, kit);
    }

    let mut foot = Vec::new();
    if state.confirm_restore {
        // The builder's one question, before a restore replaces its edits.
        let question = parts::line(
            commands,
            kit,
            Phrase::BuildDiscardQuestion.text(lang),
            kit.m.small,
            tokens::INK,
        );
        let keep = controls::button(
            commands,
            kit,
            Phrase::BuildKeepEditing.text(lang),
            Weight::Secondary,
            Live::Yes,
            None,
            Press::Decks(DecksPress::KeepEdits),
        );
        let discard = controls::button(
            commands,
            kit,
            Phrase::BuildDiscard.text(lang),
            Weight::Danger,
            Live::Yes,
            None,
            Press::Decks(DecksPress::DiscardAndRestore),
        );
        orders::stop(commands, keep, &orders::HISTORY, "close");
        orders::stop(commands, discard, &orders::HISTORY, "restore");
        foot.extend([question, keep, discard]);
    } else {
        let close = controls::button(
            commands,
            kit,
            Phrase::ShellClose.text(lang),
            Weight::Secondary,
            Live::Yes,
            None,
            Press::Decks(DecksPress::CloseSheet),
        );
        let export = controls::button(
            commands,
            kit,
            Phrase::HistoryExport.text(lang),
            Weight::Secondary,
            if lib.selected.is_some_and(|s| lib.snapshots.contains_key(&s)) {
                Live::Yes
            } else {
                Live::No(Phrase::LibraryLoading.text(lang))
            },
            None,
            Press::Decks(DecksPress::ExportVersion),
        );
        orders::stop(commands, export, &orders::HISTORY, "export");
        orders::stop(commands, close, &orders::HISTORY, "close");
        foot.extend([export, close]);
        if !house {
            let current = lib
                .selected
                .zip(lib.history.as_ref())
                .is_none_or(|(s, h)| s == h.version);
            let restore = controls::button(
                commands,
                kit,
                Phrase::HistoryRestore.text(lang),
                Weight::Primary,
                if current {
                    Live::No(Phrase::HistoryRestoreCurrent.text(lang))
                } else if lib.loading {
                    Live::No(Phrase::LibraryLoading.text(lang))
                } else {
                    Live::Yes
                },
                None,
                Press::Decks(DecksPress::Restore),
            );
            orders::stop(commands, restore, &orders::HISTORY, "restore");
            foot.push(restore);
        }
    }
    let title = if house {
        format!(
            "{} · {}",
            Phrase::HistoryTitle.fill(lang, &[&name]),
            Phrase::HistoryHouseTag.text(lang)
        )
    } else {
        Phrase::HistoryTitle.fill(lang, &[&name])
    };
    let surface = surfaces::sheet_box(commands, kit, SheetWidth::Large, &title, &[columns], &foot);
    let scrim = surfaces::sheet(commands, surface);
    commands
        .entity(scrim)
        .insert(Press::Decks(DecksPress::CloseSheet));
    commands
        .entity(surface)
        .insert((Press::Shared(SharedPress::PickerNothing), HistorySheet));
    commands.entity(root).add_child(scrim);
    // Export…: the builder's own dialog, over the sheet, on the version's
    // rows (`DeckBuilder::open_version_export`), from either door.
    let deck = lobby.builder();
    if deck.export_version().is_some()
        && let Some(open) = deck.transfer()
    {
        let dialog = crate::buildui::transfer::transfer_dialog(
            commands,
            kit.fonts,
            crate::buildui::lobby_metrics_of(kit.m),
            lang,
            deck,
            open,
            &Scrolled::default(),
        );
        // Above the sheet it stands over (a sheet's own layer, plus one).
        commands
            .entity(dialog)
            .insert(GlobalZIndex(crate::shellkit::tokens::z::SHEET + 1));
        commands.entity(root).add_child(dialog);
    }
}

/// The version the sheet shows, as the export dialog writes it: its stored
/// rows, named after the deck and the version (`Weltenbaum v7`).
pub(super) fn version_rows(
    state: &LobbyState,
) -> Option<baylee_client_core::deckbuilder::transfer::VersionRows> {
    let lobby = &state.lobby;
    let lib = lobby.library();
    let Some(Page::History(id)) = &lib.page else {
        return None;
    };
    let selected = lib.selected?;
    let snapshot = lib.snapshots.get(&selected)?;
    let name = lobby
        .decks()
        .iter()
        .find(|d| &d.id == id)
        .map(|d| d.name.clone())
        .or_else(|| {
            lib.house
                .iter()
                .find(|d| &d.id == id)
                .map(|d| d.name.clone())
        })
        .unwrap_or_default();
    Some(baylee_client_core::deckbuilder::transfer::VersionRows {
        name: format!("{name} v{selected}").trim().to_string(),
        cards: snapshot.cards.clone(),
        sideboard: snapshot.sideboard.clone(),
        commanders: snapshot.commanders.clone(),
    })
}

/// The history sheet's surface: the one sheet both doors open, where the
/// tours' `history_sheet` anchor (TOURS D14) belongs; walks and `/state`
/// find it by this.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct HistorySheet;

/// `vA → vB` and the compare bar, then the counts of both ends.
fn compare_bar(commands: &mut Commands, parent: Entity, state: &LobbyState, kit: Kit) {
    let lobby = &state.lobby;
    let lang = lobby.lang();
    let lib = lobby.library();
    let Some((from, to)) = lobby.compared() else {
        return;
    };
    let bar = commands
        .spawn((
            Node {
                align_items: AlignItems::Center,
                column_gap: kit.m.px(8.0),
                row_gap: kit.m.px(4.0),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let ends = match from {
        Some(from) if from != to => format!("v{from} \u{2192} v{to}"),
        _ => format!("v{to}"),
    };
    let label = controls::label(commands, kit, &ends, kit.m.text, tokens::INK);
    commands.entity(label).insert(HistoryEnds);
    let at = COMPARE
        .iter()
        .position(|c| std::mem::discriminant(c) == std::mem::discriminant(&lib.compare))
        .unwrap_or(0);
    let names = [
        Phrase::HistoryPrevious.text(lang),
        Phrase::HistoryCurrent.text(lang),
        Phrase::HistoryPick.text(lang),
    ];
    let segmented = controls::segmented(commands, kit, &names, at, |i| {
        Press::Decks(DecksPress::Compare(u8::try_from(i).unwrap_or(0)))
    });
    // One Tab stop; ← → walk its three and choose (`keys`).
    orders::items_of(commands, segmented, &orders::HISTORY, "compare");
    let gap = parts::grow(commands);
    let all = controls::chip(
        commands,
        kit,
        Phrase::HistoryShowAll.text(lang),
        state.decks.show_all,
        None,
        false,
        Press::Decks(DecksPress::ShowAll),
    );
    orders::stop(commands, all, &orders::HISTORY, "show-all");
    commands
        .entity(bar)
        .add_children(&[label, segmented, gap, all]);
    commands.entity(parent).add_child(bar);
    if lib.compare == Compare::Pick(None) {
        let hint = parts::line(
            commands,
            kit,
            Phrase::HistoryPicking.text(lang),
            kit.m.small,
            tokens::MUTED,
        );
        commands.entity(parent).add_child(hint);
    }
    let after = lib.snapshots.get(&to);
    let before = from.and_then(|v| lib.snapshots.get(&v)).or(after);
    if let (Some(before), Some(after)) = (before, after) {
        let counts = |s: &Snapshot| {
            format!(
                "{} · SB {}",
                deckdiff::card_count(&s.cards),
                deckdiff::card_count(&s.sideboard)
            )
        };
        let mut summary = if from.is_some_and(|f| f != to) {
            format!("{} \u{2192} {}", counts(before), counts(after))
        } else {
            counts(after)
        };
        if let Some(diff) = lobby.compared_changes() {
            let delta = total(&diff);
            if !delta.is_empty() {
                summary = format!("{summary} · {}", delta_line(&delta, lang));
            }
        }
        let line = parts::line(commands, kit, &summary, kit.m.small, tokens::MUTED);
        commands.entity(parent).add_child(line);
    }
}

/// The `vA → vB` the compare bar names (the walk reads it).
#[derive(Component, Clone, Debug)]
pub(crate) struct HistoryEnds;

/// The delta of three zones together.
fn total(diff: &[ZoneDiff; 3]) -> Delta {
    diff.iter().fold(Delta::default(), |sum, zone| {
        let d = zone.delta();
        Delta {
            added: sum.added + d.added,
            removed: sum.removed + d.removed,
            count: sum.count + d.count,
            printing: sum.printing + d.printing,
            finish: sum.finish + d.finish,
            language: sum.language + d.language,
            note: sum.note + d.note,
        }
    })
}

/// `+3 −2 · 1 printing(s) · 1 foil`: what a save changed, in one line.
pub(super) fn delta_line(delta: &Delta, lang: Lang) -> String {
    let mut parts = Vec::new();
    if delta.added > 0 || delta.removed > 0 {
        parts.push(Phrase::HistoryDeltaCopies.fill(
            lang,
            &[&delta.added.to_string(), &delta.removed.to_string()],
        ));
    }
    for (n, phrase) in [
        (delta.printing, Phrase::HistoryDeltaPrinting),
        (delta.finish, Phrase::HistoryDeltaFinish),
        (delta.language, Phrase::HistoryDeltaLanguage),
        (delta.note, Phrase::HistoryDeltaNote),
    ] {
        if n > 0 {
            parts.push(phrase.fill(lang, &[&n.to_string()]));
        }
    }
    parts.join(" · ")
}

/// The changes by zone, each row with its picture and its kind.
fn changes(commands: &mut Commands, parent: Entity, state: &LobbyState, kit: Kit) {
    let lobby = &state.lobby;
    let lang = lobby.lang();
    let lib = lobby.library();
    let Some((from, to)) = lobby.compared() else {
        return;
    };
    let Some(diff) = lobby.compared_changes() else {
        let skeleton = crate::shellkit::states::skeleton(commands, kit, 4);
        commands.entity(parent).add_child(skeleton);
        return;
    };
    let after = lib.snapshots.get(&to);
    let before = from.and_then(|v| lib.snapshots.get(&v)).or(after);
    let labels = [
        Phrase::LibraryMain,
        Phrase::LibrarySide,
        Phrase::LibraryCommanders,
    ];
    // The diff is one Tab stop whose rows ↑ ↓ walk, across the zones.
    let mut walked = 0usize;
    for (zone, (label, diff)) in labels.iter().zip(diff.iter()).enumerate() {
        let empty_both = [before, after].iter().flatten().all(|s| match zone {
            0 => s.cards.is_empty(),
            1 => s.sideboard.is_empty(),
            _ => s.commanders.is_empty(),
        });
        if zone > 0 && empty_both {
            continue;
        }
        let title = parts::caption(commands, kit, label.text(lang));
        commands.entity(parent).add_child(title);
        if diff.changes.is_empty() {
            let same = parts::line(
                commands,
                kit,
                Phrase::NoChanges.text(lang),
                kit.m.small,
                tokens::MUTED,
            );
            commands.entity(parent).add_child(same);
        }
        for change in &diff.changes {
            let row = change_row(commands, kit, state, change);
            orders::item(commands, row, &orders::HISTORY, "diff", walked);
            walked += 1;
            commands.entity(parent).add_child(row);
        }
    }
    if state.decks.show_all
        && let (Some(before), Some(after)) = (before, after)
    {
        all_cards(commands, parent, kit, lang, before, after, from.is_some());
    }
}

/// One classified change: thumbnail, words, the kind's tag.
fn change_row(commands: &mut Commands, kit: Kit, state: &LobbyState, change: &Change) -> Entity {
    let lang = state.lobby.lang();
    let (text, ink, tag) = describe(change, lang);
    let hover = hover_of(state, change.name(), change.print());
    let row = commands
        .spawn((
            Node {
                align_items: AlignItems::Center,
                column_gap: kit.m.px(10.0),
                min_height: px_fixed(kit.m.row.min(56.0)),
                width: Val::Percent(100.0),
                ..default()
            },
            HistoryChange,
            Press::Decks(DecksPress::DiffRow),
        ))
        .id();
    let thumb = super::thumbnails::spawn(commands, &hover);
    let words = controls::label(commands, kit, &text, kit.m.text, ink);
    commands.entity(words).insert(Node {
        flex_grow: 1.0,
        flex_shrink: 1.0,
        min_width: px(0),
        ..default()
    });
    let tag = controls::label(commands, kit, tag, kit.m.small, tokens::MUTED);
    commands.entity(row).add_children(&[thumb, words, tag]);
    commands.entity(row).insert(hover);
    row
}

/// A row of the classified diff (for walks and `/state`).
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct HistoryChange;

/// A change's words, ink and tag.
fn describe(change: &Change, lang: Lang) -> (String, Color, &'static str) {
    match change {
        Change::Added { name, count, .. } => (
            format!("+ {count}  {name}"),
            tokens::ACCENT,
            Phrase::HistoryTagAdded.text(lang),
        ),
        Change::Removed { name, count, .. } => (
            format!("\u{2212} {count}  {name}"),
            tokens::DANGER,
            Phrase::HistoryTagRemoved.text(lang),
        ),
        Change::Count { name, from, to, .. } => (
            format!("{from} \u{2192} {to}  {name}"),
            tokens::INK,
            Phrase::HistoryTagCount.text(lang),
        ),
        Change::Printing { name, from, to, .. } => (
            format!("{name} · {} \u{2192} {}", printing(from), printing(to)),
            tokens::INK,
            Phrase::HistoryTagPrinting.text(lang),
        ),
        Change::Finish { name, from, to, .. } => (
            format!(
                "{name} · {} \u{2192} {}",
                finish_name(*from, lang),
                finish_name(*to, lang)
            ),
            tokens::INK,
            Phrase::HistoryTagFinish.text(lang),
        ),
        Change::Language { name, from, to, .. } => (
            format!("{name} · {from} \u{2192} {to}"),
            tokens::INK,
            Phrase::HistoryTagLanguage.text(lang),
        ),
        Change::Note { name, .. } => (
            Phrase::HistoryNoteChanged.fill(lang, &[name]),
            tokens::INK,
            Phrase::HistoryTagNote.text(lang),
        ),
    }
}

/// `M11 #149`, or `M11`, or the exact id's head, or nothing.
fn printing(print: &PrintChoice) -> String {
    match (&print.set, &print.collector_number, &print.scryfall_id) {
        (Some(set), Some(number), _) => format!("{} #{number}", set.to_uppercase()),
        (Some(set), None, _) => set.to_uppercase(),
        (None, _, Some(id)) => id.chars().take(8).collect(),
        _ => "\u{2014}".to_string(),
    }
}

/// A finish's name, as the printing picker says it.
fn finish_name(finish: Finish, lang: Lang) -> &'static str {
    match finish {
        Finish::Normal => Phrase::FinishPlain,
        Finish::Foil => Phrase::FinishFoil,
        Finish::Etched => Phrase::FinishEtched,
        Finish::Holographic => Phrase::FinishHolographic,
        Finish::Glitter => Phrase::FinishGlitter,
        Finish::Galaxy => Phrase::FinishGalaxy,
    }
    .text(lang)
}

/// The picture a diff row shows and previews: the row's printing, else the
/// pool's reference printing for the bare name, else none (a text face).
fn hover_of(state: &LobbyState, name: &str, print: &PrintChoice) -> HoverCard {
    let pool = state.lobby.builder().pool();
    if let Some(card) = pool.iter().find(|c| c.name == name) {
        return super::preview::hover_of_entry(card, print);
    }
    let url = print.scryfall_id.as_ref().and_then(|id| {
        let entry = baylee_view::PrintEntry {
            scryfall_id: id.clone(),
            lang: print.lang_or_default().to_string(),
            finish: baylee_view::Finish::Normal,
        };
        client_core::images::image_url(
            &entry,
            client_core::images::Face::Front,
            client_core::images::ArtSize::Normal,
        )
    });
    HoverCard {
        url,
        back_url: None,
        finish: client_core::images::FinishTreatment::Plain,
        index: None,
    }
}

/// Show all cards: both ends by zone, left the older, right the newer,
/// each row as `n Name · SET #num · Foil · ja`, never the raw row string.
fn all_cards(
    commands: &mut Commands,
    parent: Entity,
    kit: Kit,
    lang: Lang,
    before: &Snapshot,
    after: &Snapshot,
    two: bool,
) {
    let both = commands
        .spawn((
            Node {
                column_gap: kit.m.px(16.0),
                width: Val::Percent(100.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let ends: Vec<&Snapshot> = if two && before.version != after.version {
        vec![before, after]
    } else {
        vec![after]
    };
    for snapshot in ends {
        let column = commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    flex_basis: px(0),
                    min_width: px(0),
                    row_gap: kit.m.px(2.0),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let head = parts::caption(commands, kit, &format!("v{}", snapshot.version));
        commands.entity(column).add_child(head);
        for (label, rows) in [
            (Phrase::LibraryCommanders, &snapshot.commanders),
            (Phrase::LibraryMain, &snapshot.cards),
            (Phrase::LibrarySide, &snapshot.sideboard),
        ] {
            if rows.is_empty() {
                continue;
            }
            let title = parts::caption(commands, kit, label.text(lang));
            let text: Vec<String> = rows.iter().map(|r| readable(r, lang)).collect();
            let text = parts::line(commands, kit, &text.join("\n"), kit.m.small, tokens::INK);
            commands.entity(column).add_children(&[title, text]);
        }
        commands.entity(both).add_child(column);
    }
    commands.entity(parent).add_child(both);
}

/// A stored row as a player reads it.
fn readable(row: &str, lang: Lang) -> String {
    let Ok(parsed) = deckrow::parse(row) else {
        return row.to_string();
    };
    let mut words = vec![format!("{} {}", parsed.count, parsed.name)];
    let print = printing(&parsed.print);
    if parsed.print.set.is_some() || parsed.print.scryfall_id.is_some() {
        words.push(print);
    }
    if parsed.print.finish.is_some_and(|f| f != Finish::Normal) {
        words.push(finish_name(parsed.print.finish_or_default(), lang).to_string());
    }
    if let Some(code) = &parsed.print.lang
        && code != "en"
    {
        words.push(code.clone());
    }
    words.join(" · ")
}

/// A version's time on the device's clock: `today 14:02`, `yesterday
/// 21:40`, else `2026-10-02 18:05` (the log's clock, `parts::now_secs`).
pub(super) fn when(at: i64, lang: Lang) -> String {
    let Ok(utc) = time::OffsetDateTime::from_unix_timestamp(at) else {
        return String::new();
    };
    let offset = time::UtcOffset::local_offset_at(utc).unwrap_or(time::UtcOffset::UTC);
    let at = utc.to_offset(offset);
    let now = i64::try_from(parts::now_secs())
        .ok()
        .and_then(|now| time::OffsetDateTime::from_unix_timestamp(now).ok())
        .map(|now| now.to_offset(offset));
    let clock = format!("{:02}:{:02}", at.hour(), at.minute());
    match now.map(|now| (now.date() - at.date()).whole_days()) {
        Some(0) => format!("{} {clock}", Phrase::HistoryToday.text(lang)),
        Some(1) => format!("{} {clock}", Phrase::HistoryYesterday.text(lang)),
        _ => format!(
            "{:04}-{:02}-{:02} {clock}",
            at.year(),
            u8::from(at.month()),
            at.day()
        ),
    }
}

/// The history's keys (`DESIGN` §C.3 Keyboard): `↑↓` on the versions move
/// the selection and the diff follows, `←→` on the compare bar choose,
/// `Esc` leaves the Pick… mode, then the discard question, then the sheet.
/// `on` is the focused stop of the sheet's order. Returns what to send.
pub(super) fn keys(
    codes: &ButtonInput<KeyCode>,
    state: &mut LobbyState,
    on: Option<&str>,
) -> Option<client_core::lobby::LobbyRequest> {
    if codes.just_pressed(KeyCode::Escape) {
        if state.confirm_restore {
            state.confirm_restore = false;
        } else if !state.lobby.leave_pick() {
            state.lobby.close_library();
        }
        return None;
    }
    // Each composite owns its arrows: ↑ ↓ the versions (also with no ring
    // up, as a pointer leaves it), ← → the compare bar; the diff's rows are
    // the walker's alone.
    let versions = on.is_none_or(|id| id == "versions");
    if versions && codes.just_pressed(KeyCode::ArrowDown) {
        return state.lobby.step_version(1);
    }
    if versions && codes.just_pressed(KeyCode::ArrowUp) {
        return state.lobby.step_version(-1);
    }
    if on != Some("compare") {
        return None;
    }
    let by = if codes.just_pressed(KeyCode::ArrowRight) {
        1
    } else if codes.just_pressed(KeyCode::ArrowLeft) {
        2
    } else {
        return None;
    };
    let at = COMPARE
        .iter()
        .position(|c| {
            std::mem::discriminant(c) == std::mem::discriminant(&state.lobby.library().compare)
        })
        .unwrap_or(0);
    state.lobby.compare_with(COMPARE[(at + by) % COMPARE.len()])
}
