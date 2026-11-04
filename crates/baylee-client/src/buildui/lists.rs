//! Ban lists over the builder: the chips that hold a deck to one, and the
//! sheet that reads or edits one.
//!
//! `banlist::ListEditor` decides; this draws. The search in the sheet is the
//! builder's box over the builder's pool, so it reads what every card search
//! reads, the client's language and English.

#[allow(clippy::wildcard_imports)] // the builder's own vocabulary
use super::*;
use crate::lobby::{FieldLook, List, Scrollable};
use crate::shellkit::surfaces::{self, SheetWidth};
use baylee_cardtext::ListStatus;
use baylee_client_core::banlist::ListEditor;
use baylee_client_core::deckbuilder::BuildField;
use baylee_client_core::deckcheck::format_name;

/// What a status is called.
const fn status_phrase(status: ListStatus) -> Phrase {
    match status {
        ListStatus::Banned => Phrase::ListStatusBanned,
        ListStatus::Restricted => Phrase::ListStatusRestricted,
        ListStatus::BannedAsCommander => Phrase::ListStatusBannedAsCommander,
    }
}

/// The check sheet's part: none or a list, and what can be done with it.
pub(super) fn picker(commands: &mut Commands, env: &Env, body: &mut Vec<Entity>) {
    let kit = env.kit;
    let lang = env.lang();
    let deck = env.deck();
    if env.state.lobby.token().is_none() {
        return;
    }
    body.push(caption(commands, kit, Phrase::CheckList.text(lang)));
    let held = deck.list().map(|list| list.id.as_str());
    let mut chips = vec![controls::chip(
        commands,
        kit,
        Phrase::CheckNoList.text(lang),
        held.is_none(),
        None,
        false,
        (
            Press::Build(BuildPress::ListChoose(None)),
            sheet_stop("list"),
        ),
    )];
    for (at, row) in deck.lists().iter().enumerate() {
        chips.push(controls::chip(
            commands,
            kit,
            &deck.list_name(row, lang),
            held == Some(row.id.as_str()),
            None,
            false,
            (
                Press::Build(BuildPress::ListChoose(Some(at))),
                sheet_stop("list"),
            ),
        ));
    }
    body.push(super::legality::wrapped(commands, kit, &chips));
    let mut actions = vec![link(
        commands,
        kit,
        Phrase::ListNew.text(lang),
        (Press::Build(BuildPress::ListNew), sheet_stop("list-new")),
    )];
    let chosen = held.and_then(|id| deck.lists().iter().position(|row| row.id == id));
    if let Some(at) = chosen {
        let word = if deck.lists()[at].mine {
            Phrase::ListEdit
        } else {
            Phrase::ListRead
        };
        actions.push(link(
            commands,
            kit,
            word.text(lang),
            (
                Press::Build(BuildPress::ListOpen(at)),
                sheet_stop("list-open"),
            ),
        ));
        actions.push(link(
            commands,
            kit,
            Phrase::ListCopy.text(lang),
            (
                Press::Build(BuildPress::ListCopyOf(at)),
                sheet_stop("list-copy"),
            ),
        ));
    }
    body.push(line(commands, kit, &actions));
}

/// The list open in the editor, read or written.
#[allow(clippy::too_many_lines)] // one sheet, read top to bottom
pub(super) fn sheet(commands: &mut Commands, env: &Env, list: &ListEditor) -> Entity {
    let kit = env.kit;
    let lang = env.lang();
    let deck = env.deck();
    let mut body = Vec::new();
    if list.read_only() {
        let said = format!(
            "{} · {}",
            Phrase::ListOfficial.text(lang),
            format_name(list.format(), lang)
        );
        body.push(cell(commands, kit, &said, kit.m.text, tokens::MUTED, false));
    } else {
        body.push(crate::lobby::text_field(
            commands,
            kit.fonts,
            env.lobby_metrics(),
            Phrase::ListNameLabel.text(lang),
            &FieldLook {
                buffer: deck.buffer(BuildField::ListName),
                focused: deck.focus() == BuildField::ListName,
                mask: None,
                press: Press::Build(BuildPress::FocusBuild(BuildField::ListName)),
                lead: None,
                hint: None,
                tail: None,
            },
        ));
        body.push(caption(commands, kit, Phrase::CheckFormat.text(lang)));
        let chips: Vec<Entity> = deck
            .format_choices()
            .iter()
            .enumerate()
            .map(|(at, key)| {
                controls::chip(
                    commands,
                    kit,
                    format_name(key, lang),
                    key == list.format(),
                    None,
                    false,
                    (
                        Press::Build(BuildPress::ListFormat(at)),
                        sheet_stop("format"),
                    ),
                )
            })
            .collect();
        body.push(super::legality::wrapped(commands, kit, &chips));
        body.push(crate::lobby::text_field(
            commands,
            kit.fonts,
            env.lobby_metrics(),
            Phrase::ListSearchLabel.text(lang),
            &FieldLook {
                buffer: deck.buffer(BuildField::ListSearch),
                focused: deck.focus() == BuildField::ListSearch,
                mask: None,
                press: Press::Build(BuildPress::FocusBuild(BuildField::ListSearch)),
                lead: Some(crate::hud::glyph::MAGNIFIER),
                hint: Some(Phrase::SearchCards.text(lang)),
                tail: None,
            },
        ));
        for &slot in list.results() {
            if let Some(card) = deck.card(slot) {
                body.push(result_row(commands, env, slot, &card.name));
            }
        }
    }
    body.push(caption(commands, kit, Phrase::ListTitle.text(lang)));
    if list.entries().is_empty() {
        body.push(cell(
            commands,
            kit,
            Phrase::ListEmpty.text(lang),
            kit.m.small,
            tokens::MUTED,
            false,
        ));
    }
    for (at, entry) in list.entries().iter().enumerate() {
        body.push(entry_row(commands, env, list, at, entry));
    }
    let mut footer = Vec::new();
    if !list.read_only() {
        if list.id().is_some() {
            footer.push(controls::button(
                commands,
                kit,
                Phrase::ListDelete.text(lang),
                Weight::Danger,
                Live::Yes,
                None,
                (
                    Press::Build(BuildPress::ListDelete),
                    sheet_stop("list-delete"),
                ),
            ));
        }
        footer.push(controls::button(
            commands,
            kit,
            Phrase::ListSave.text(lang),
            Weight::Primary,
            if list.saveable() {
                Live::Yes
            } else {
                Live::No(Phrase::ListSave.text(lang))
            },
            None,
            (Press::Build(BuildPress::ListSave), sheet_stop("list-save")),
        ));
    }
    footer.push(controls::button(
        commands,
        kit,
        Phrase::ShellClose.text(lang),
        Weight::Secondary,
        Live::Yes,
        None,
        (Press::Build(BuildPress::ListClose), sheet_stop("close")),
    ));
    let title = if list.read_only() {
        Phrase::ListTitleOfficial.text(lang).to_string()
    } else {
        Phrase::ListTitle.text(lang).to_string()
    };
    surfaces::sheet_box_listed(
        commands,
        kit,
        SheetWidth::Medium,
        &title,
        &body,
        &footer,
        Scrollable(List::Check),
    )
}

/// A row of the search: the card, and the three words a press puts on it.
fn result_row(commands: &mut Commands, env: &Env, slot: usize, name: &str) -> Entity {
    let kit = env.kit;
    let lang = env.lang();
    let mut kids = vec![cell(commands, kit, name, kit.m.text, tokens::INK, false)];
    kids.push(spring(commands));
    for status in [
        ListStatus::Banned,
        ListStatus::Restricted,
        ListStatus::BannedAsCommander,
    ] {
        kids.push(link(
            commands,
            kit,
            status_phrase(status).text(lang),
            (
                Press::Build(BuildPress::ListPut(slot, status)),
                sheet_stop("list-put"),
            ),
        ));
    }
    line(commands, kit, &kids)
}

/// A card on the list: its name, its word (a press moves it on) and a cross.
fn entry_row(
    commands: &mut Commands,
    env: &Env,
    list: &ListEditor,
    at: usize,
    entry: &baylee_cardtext::ListEntry,
) -> Entity {
    let kit = env.kit;
    let lang = env.lang();
    let name = if entry.name.is_empty() {
        entry.oracle_id.as_str()
    } else {
        entry.name.as_str()
    };
    let mut kids = vec![cell(commands, kit, name, kit.m.text, tokens::INK, false)];
    kids.push(spring(commands));
    let word = status_phrase(entry.status).text(lang);
    if list.read_only() {
        kids.push(cell(commands, kit, word, kit.m.small, tokens::MUTED, false));
    } else {
        kids.push(link(
            commands,
            kit,
            word,
            (
                Press::Build(BuildPress::ListCycle(at)),
                sheet_stop("list-word"),
            ),
        ));
        kids.push(icon_button(
            commands,
            kit,
            mark::CLOSE,
            false,
            (
                Press::Build(BuildPress::ListRemove(at)),
                sheet_stop("list-remove"),
            ),
        ));
    }
    line(commands, kit, &kids)
}
