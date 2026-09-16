//! Opening a table and getting into somebody else's: what a `CreateGame` or a `JoinGame` carries, the deck that has to be picked before either is allowed, the chair count clamped to the bound the gateway would enforce anyway, and the room password typed once and spent on whichever of the two comes first. The host's three further statements — ready, start, hand over — are named here too, each obeying the one-request-in-flight rule that stops a double tap on Start ordering two engines. What the granted seat is then worth is `seating`; what a listed room says about itself, and how the list is paged, is `table_list`.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

#[test]
fn a_table_cannot_be_opened_without_a_deck() {
    let mut lobby = seated_lobby();
    lobby.apply(LobbyEvent::Decks(vec![]));
    lobby.apply(LobbyEvent::Games(GameListing::default()));
    assert_eq!(lobby.selected(), None);
    assert_eq!(lobby.host(GameMode::Ai), None);
    assert_eq!(lobby.status(), "pick a deck first");
    assert!(!lobby.busy(), "a refused intent leaves nothing in flight");
}

#[test]
fn hosting_names_the_selected_deck() {
    let mut lobby = seated_lobby();
    assert_eq!(
        lobby.host(GameMode::Ai),
        Some(LobbyRequest::CreateGame {
            deck_id: "d1".to_string(),
            mode: GameMode::Ai,
            chairs: 2,
            name: String::new(),
            password: String::new(),
        })
    );
}

#[test]
fn only_one_request_is_in_flight_at_a_time() {
    let mut lobby = seated_lobby();
    assert!(lobby.host(GameMode::Open).is_some());
    assert_eq!(lobby.host(GameMode::Open), None, "a second click is idle");
    assert_eq!(lobby.join("g1"), None);
    assert_eq!(lobby.refresh(), None);
}

#[test]
fn joining_brings_the_selected_deck_to_someone_elses_table() {
    let mut lobby = seated_lobby();
    assert_eq!(
        lobby.join("g7"),
        Some(LobbyRequest::JoinGame {
            game_id: "g7".to_string(),
            deck_id: "d1".to_string(),
            seat: None,
            password: String::new(),
        })
    );
}

/// The box is typed into once and spent once — on opening a room or on
/// joining one, whichever comes first. A password left lying in a text
/// box is the next room's password by accident.
#[test]
fn the_room_password_goes_with_the_next_table_and_is_then_forgotten() {
    let mut lobby = seated_lobby();
    lobby.focus_on(Field::RoomPassword);
    for ch in "kitchen".chars() {
        lobby.type_char(ch);
    }
    assert_eq!(lobby.room_password(), "kitchen");
    assert_eq!(
        lobby.join_seat("g7", Some(2)),
        Some(LobbyRequest::JoinGame {
            game_id: "g7".to_string(),
            deck_id: "d1".to_string(),
            seat: Some(2),
            password: "kitchen".to_string(),
        })
    );
    assert!(lobby.room_password().is_empty(), "and it is gone");

    lobby.apply(LobbyEvent::Failed("wrong password".to_string()));
    lobby.focus_on(Field::RoomPassword);
    for ch in "supper".chars() {
        lobby.type_char(ch);
    }
    assert_eq!(
        lobby.open_room(GameMode::Open, 3, "Kitchen".to_string()),
        Some(LobbyRequest::CreateGame {
            deck_id: "d1".to_string(),
            mode: GameMode::Open,
            chairs: 3,
            name: "Kitchen".to_string(),
            password: "supper".to_string(),
        })
    );
    assert!(lobby.room_password().is_empty());
}

/// Ready, start and handover are three different statements, and the two
/// that are the host's are not the one that is the player's.
#[test]
fn a_room_is_readied_started_and_handed_on_by_name() {
    let mut lobby = seated_lobby();
    assert_eq!(
        lobby.set_ready("g7", true),
        Some(LobbyRequest::SetReady {
            game_id: "g7".to_string(),
            ready: true,
        })
    );
    lobby.apply(LobbyEvent::Games(GameListing::default()));
    assert_eq!(
        lobby.set_ready("g7", false),
        Some(LobbyRequest::SetReady {
            game_id: "g7".to_string(),
            ready: false,
        })
    );
    lobby.apply(LobbyEvent::Games(GameListing::default()));
    assert_eq!(
        lobby.start_room("g7"),
        Some(LobbyRequest::StartGame {
            game_id: "g7".to_string(),
        })
    );
    lobby.apply(LobbyEvent::Games(GameListing::default()));
    assert_eq!(
        lobby.hand_over("g7", 2),
        Some(LobbyRequest::HandOver {
            game_id: "g7".to_string(),
            seat: 2,
        })
    );
    // And all three obey the one-request-in-flight rule, so a double tap
    // on "start" cannot order two engines.
    assert_eq!(lobby.start_room("g7"), None);
}

/// The size a room is opened at is clamped to what the gateway accepts,
/// so a client can never ask for a table that would be refused.
#[test]
fn a_room_is_opened_at_a_size_the_gateway_allows() {
    let mut lobby = seated_lobby();
    for (asked, expected) in [(1, MIN_CHAIRS), (3, 3), (9, MAX_CHAIRS)] {
        // Each open_room marks the lobby busy; the answer clears it.
        lobby.apply(LobbyEvent::Games(GameListing::default()));
        let Some(LobbyRequest::CreateGame { chairs, .. }) =
            lobby.open_room(GameMode::Open, asked, "Kitchen".to_string())
        else {
            panic!("a picked deck opens a room");
        };
        assert_eq!(chairs, expected, "asked for {asked}");
    }
}
