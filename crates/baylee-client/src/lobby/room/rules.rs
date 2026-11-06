//! What a room holds a deck to, on the screen: its line in the rules, the
//! host's choice, and the list for those at the table.
//!
//! The gateway decides who may see what (`docs/protocol.md` §"Ban lists"): an
//! own list has no name to someone not at the table, and a running game
//! shows the version it froze.

#[allow(clippy::wildcard_imports)] // the room's own vocabulary
use super::*;
use crate::shellkit::surfaces::{self, SheetWidth};
use baylee_client_core::deckcheck::format_name;

/// The room's line about its list, as this reader may see it.
pub(super) fn line_of(state: &LobbyState, game: &GameSummary) -> String {
    let lobby = &state.lobby;
    let lang = lobby.lang();
    match &game.rules.list {
        None => Phrase::RoomRulesNoList.text(lang).to_string(),
        Some(list) => lobby.table_list_name(list).map_or_else(
            || Phrase::RoomRulesOwnList.text(lang).to_string(),
            |name| Phrase::RoomRulesList.fill(lang, &[&name]),
        ),
    }
}

/// The rail's part: the line, and what the reader may do about it.
pub(super) fn draw(
    commands: &mut Commands,
    rail: Entity,
    state: &LobbyState,
    kit: Kit,
    index: usize,
    game: &GameSummary,
) {
    let lang = state.lobby.lang();
    let said = super::super::parts::line(
        commands,
        kit,
        &line_of(state, game),
        kit.m.text,
        tokens::INK,
    );
    commands.entity(rail).add_child(said);
    if let Some(list) = game.rules.list.as_ref().filter(|list| list.frozen)
        && let Some(version) = list.version
    {
        let frozen = Phrase::RoomRulesFrozen.fill(lang, &[&version.to_string()]);
        let frozen = super::super::parts::line(commands, kit, &frozen, kit.m.small, tokens::MUTED);
        commands.entity(rail).add_child(frozen);
    }
    let waiting = game.state == "waiting";
    // Lists live on a gateway, so an offline room offers none.
    if game.yours && waiting && state.offline.is_none() {
        let choose = controls::button(
            commands,
            kit,
            Phrase::RoomRulesChoose.text(lang),
            Weight::Ghost,
            Live::Yes,
            None,
            Press::Room(RoomPress::OpenLists),
        );
        commands.entity(rail).add_child(choose);
    }
    let here = game.seats.iter().any(|seat| seat.you);
    if here && game.rules.list.is_some() {
        let read = controls::button(
            commands,
            kit,
            Phrase::RoomRulesRead.text(lang),
            Weight::Ghost,
            Live::Yes,
            None,
            Press::Room(RoomPress::ReadList(index)),
        );
        commands.entity(rail).add_child(read);
    }
}

/// The host's choice: no list, an official one, or one of their own.
pub(super) fn chooser(
    commands: &mut Commands,
    root: Entity,
    state: &LobbyState,
    kit: Kit,
    index: usize,
) {
    let lang = state.lobby.lang();
    let deck = state.lobby.builder();
    let mut body = Vec::new();
    let none = controls::chip(
        commands,
        kit,
        Phrase::ListNone.text(lang),
        state.lobby.games()[index].rules.list.is_none(),
        None,
        false,
        Press::Room(RoomPress::ChooseList(index, None)),
    );
    body.push(none);
    for (at, row) in deck.lists().iter().enumerate() {
        let chip = controls::chip(
            commands,
            kit,
            &deck.list_name(row, lang),
            false,
            None,
            false,
            Press::Room(RoomPress::ChooseList(index, Some(at))),
        );
        body.push(chip);
    }
    let close = controls::button(
        commands,
        kit,
        Phrase::ShellClose.text(lang),
        Weight::Primary,
        Live::Yes,
        None,
        Press::Room(RoomPress::CloseLists),
    );
    // Twenty-odd lists stand taller than a window, a player's own last.
    let surface = surfaces::sheet_box_listed(
        commands,
        kit,
        SheetWidth::Small,
        Phrase::RoomListTitle.text(lang),
        &body,
        &[close],
        super::super::Scrollable(List::Check),
    );
    let scrim = surfaces::sheet(commands, surface);
    commands
        .entity(scrim)
        .insert(Press::Room(RoomPress::CloseLists));
    commands
        .entity(surface)
        .insert(Press::Shared(SharedPress::PickerNothing));
    commands.entity(root).add_child(scrim);
}

/// The table's list and its cards, for those at the table.
pub(super) fn sheet(commands: &mut Commands, root: Entity, state: &LobbyState, kit: Kit) {
    let Some(list) = state.lobby.room_list() else {
        return;
    };
    let lang = state.lobby.lang();
    let name = if list.row.builtin {
        format_name(&list.row.format, lang).to_string()
    } else {
        list.row.name.clone()
    };
    let mut body = vec![super::super::parts::line(
        commands,
        kit,
        &name,
        kit.m.text,
        tokens::INK,
    )];
    if list.entries.is_empty() {
        body.push(super::super::parts::line(
            commands,
            kit,
            Phrase::ListEmpty.text(lang),
            kit.m.small,
            tokens::MUTED,
        ));
    }
    for entry in &list.entries {
        let word = match entry.status {
            baylee_cardtext::ListStatus::Banned => Phrase::ListStatusBanned,
            baylee_cardtext::ListStatus::Restricted => Phrase::ListStatusRestricted,
            baylee_cardtext::ListStatus::BannedAsCommander => Phrase::ListStatusBannedAsCommander,
        };
        let said = format!("{} · {}", entry.name, word.text(lang));
        body.push(super::super::parts::line(
            commands,
            kit,
            &said,
            kit.m.small,
            tokens::INK,
        ));
    }
    let close = controls::button(
        commands,
        kit,
        Phrase::ShellClose.text(lang),
        Weight::Primary,
        Live::Yes,
        None,
        Press::Room(RoomPress::CloseList),
    );
    let surface = surfaces::sheet_box_listed(
        commands,
        kit,
        SheetWidth::Small,
        Phrase::RoomListTitle.text(lang),
        &body,
        &[close],
        super::super::Scrollable(List::Check),
    );
    let scrim = surfaces::sheet(commands, surface);
    commands
        .entity(scrim)
        .insert(Press::Room(RoomPress::CloseList));
    commands
        .entity(surface)
        .insert(Press::Shared(SharedPress::PickerNothing));
    commands.entity(root).add_child(scrim);
}
