//! Ban lists through the lobby: what each answer does to it, and what a
//! refusal says.

use super::*;
use baylee_cardtext::{DeckRefusal, ListBody, ListRow, Problem};

fn body(id: &str, mine: bool) -> ListBody {
    ListBody {
        row: ListRow {
            id: id.to_string(),
            name: "Kitchen table".to_string(),
            format: "modern".to_string(),
            mine,
            ..ListRow::default()
        },
        entries: Vec::new(),
    }
}

/// A table's list turned a deck away: the reasons are kept for the sheet, a
/// sentence each, and the lobby is free again.
#[test]
fn a_refusal_is_kept_and_read_in_the_players_words() {
    let mut lobby = seated_lobby();
    lobby.apply(LobbyEvent::DeckRefused(DeckRefusal {
        list: "No rings".to_string(),
        format: "commander".to_string(),
        problems: vec![
            Problem::WrongSize {
                have: 60,
                need: 100,
            },
            Problem::ListBanned {
                card: baylee_cards::decks::by_name("Sol Ring")
                    .expect("in the pool")
                    .get(),
            },
        ],
    }));
    assert!(!lobby.busy(), "the answer ended the request");
    let lines = lobby.refusal_lines();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("100"), "{lines:?}");
    assert!(lines[1].contains("Sol Ring"), "{lines:?}");
    assert!(lines[1].contains("No rings"), "{lines:?}");
    lobby.close_refusal();
    assert!(lobby.refusal().is_none());
    assert!(lobby.refusal_lines().is_empty());
}

/// The lists land on the builder, whose chips and the room's choice read them.
#[test]
fn the_lists_land_on_the_builder() {
    let mut lobby = seated_lobby();
    lobby.apply(LobbyEvent::Banlists(vec![body("a", true).row]));
    assert_eq!(lobby.builder().lists().len(), 1);
}

#[test]
fn a_list_asked_for_the_deck_holds_it_and_one_for_the_editor_opens() {
    let mut lobby = seated_lobby();
    assert_eq!(
        lobby.apply(LobbyEvent::Banlist(body("a", true), ListFor::Deck)),
        None
    );
    assert_eq!(
        lobby.builder().list().map(|list| list.id.as_str()),
        Some("a")
    );
    assert_eq!(
        lobby.apply(LobbyEvent::Banlist(body("a", true), ListFor::Editor)),
        Some(LobbyRequest::ListBanlists),
        "a list written may have a new name: the chips read it again"
    );
    assert!(lobby.builder().list_editor().is_some());

    assert_eq!(
        lobby.apply(LobbyEvent::BanlistDeleted("a".to_string())),
        Some(LobbyRequest::ListBanlists)
    );
    assert!(lobby.builder().list_editor().is_none());
    assert!(lobby.builder().list().is_none(), "and from under the deck");
}

/// An editor saved opens on what the gateway answered, with its own id: the
/// next save is an edit and not a second list.
#[test]
fn a_new_list_takes_its_id_from_the_answer() {
    let mut lobby = seated_lobby();
    lobby.builder_mut().open_new_list("Mine");
    let open = lobby.builder().list_editor().expect("an editor");
    assert_eq!(open.id(), None);
    assert!(open.save().is_some(), "a new list may be saved at once");
    lobby.apply(LobbyEvent::Banlist(body("new", true), ListFor::Editor));
    assert_eq!(
        lobby.builder().list_editor().and_then(|list| list.id()),
        Some("new")
    );
}

#[test]
fn a_room_list_is_held_until_it_is_put_away() {
    let mut lobby = seated_lobby();
    lobby.apply(LobbyEvent::RoomList(Some(body("a", false))));
    assert!(lobby.room_list().is_some());
    lobby.close_room_list();
    assert!(lobby.room_list().is_none());
}

#[test]
fn a_table_list_is_named_as_the_caller_may_read_it() {
    let lobby = seated_lobby();
    let official = TableList {
        builtin: true,
        name: Some("modern".to_string()),
        ..TableList::default()
    };
    assert_eq!(lobby.table_list_name(&official).as_deref(), Some("Modern"));
    let hidden = TableList::default();
    assert_eq!(lobby.table_list_name(&hidden), None, "not at the table");
}

/// A deck the gateway calls illegal wears the word, and only then.
#[test]
fn a_deck_tile_says_illegal_and_only_then() {
    let mut deck = DeckSummary::default();
    assert!(
        !shelf::badges(Lang::En, &deck, false)
            .iter()
            .any(|badge| badge == "Not legal")
    );
    deck.legality.standing = "illegal".to_string();
    assert!(
        shelf::badges(Lang::En, &deck, false)
            .iter()
            .any(|badge| badge == "Not legal")
    );
    deck.legality.standing = "unknown".to_string();
    assert!(
        !shelf::badges(Lang::En, &deck, false)
            .iter()
            .any(|badge| badge == "Not legal"),
        "a deck nobody could check is not called anything"
    );
}
