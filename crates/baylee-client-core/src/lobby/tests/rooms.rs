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
            clock: None,
            ai: None,
            spectators: true,
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
            clock: None,
            ai: None,
            spectators: true,
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

/// A gateway with no agent (a server being updated) says so in its listing,
/// and neither opening nor starting a room is offered until it has one
/// again. A gateway that never sends the field is read as having one.
#[test]
fn no_room_opens_or_starts_while_the_gateway_has_no_agent() {
    let silent: GameListing = serde_json::from_str(r#"{"games":[],"total":0}"#).unwrap();
    assert!(silent.agents_available, "silence means yes");

    let mut lobby = seated_lobby();
    let none: GameListing =
        serde_json::from_str(r#"{"games":[],"total":0,"agents_available":false}"#).unwrap();
    lobby.apply(LobbyEvent::Games(none));
    assert!(!lobby.games_can_start());
    assert_eq!(lobby.host(GameMode::Ai), None);
    assert_eq!(lobby.status(), Phrase::NoNewGames.text(lobby.lang()));
    assert_eq!(lobby.tone(), Tone::Refusal);
    assert_eq!(lobby.start_room("g7"), None);
    assert!(!lobby.busy(), "a refused intent leaves nothing in flight");

    lobby.apply(LobbyEvent::Games(GameListing::default()));
    assert!(lobby.games_can_start());
    assert!(lobby.host(GameMode::Ai).is_some());
}

/// A gateway that stops answering is said to have stopped, and the first
/// answer after that, whatever it is, says it is back.
#[test]
fn a_silent_gateway_is_named_until_it_answers_again() {
    let mut lobby = seated_lobby();
    assert!(lobby.refresh().is_some());
    lobby.apply(LobbyEvent::GatewayLost);
    assert!(lobby.unreachable());
    assert!(!lobby.busy(), "a lost request is not one in flight");
    assert_eq!(
        lobby.status(),
        Phrase::GatewayUnreachable.text(lobby.lang())
    );
    assert_eq!(lobby.tone(), Tone::Refusal);
    lobby.apply(LobbyEvent::GatewayLost);
    assert!(lobby.unreachable(), "a second loss changes nothing");

    lobby.apply(LobbyEvent::Games(GameListing::default()));
    assert!(!lobby.unreachable());
    assert_eq!(lobby.status(), Phrase::GatewayBack.text(lobby.lang()));
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

#[test]
fn a_room_can_be_opened_before_choosing_a_deck() {
    let mut lobby = seated_lobby();
    lobby.apply(LobbyEvent::Decks(vec![]));
    lobby.apply(LobbyEvent::Games(GameListing::default()));
    assert!(
        matches!(lobby.host(GameMode::Open), Some(LobbyRequest::CreateGame { deck_id, .. }) if deck_id.is_empty())
    );
}

#[test]
fn room_drafts_preserve_edits_until_apply_and_follow_the_room_not_search() {
    use super::super::room::Adjustment;
    let mut lobby = seated_lobby();
    lobby.set_field(Field::Search, "some other room");
    lobby.host(GameMode::Open);
    lobby.apply(LobbyEvent::Seated(SeatHandover {
        game_id: "room".into(),
        seat: 0,
        seat_token: "ticket".into(),
        local: false,
    }));
    let listing = GameListing::of(vec![GameSummary {
        id: "room".into(),
        name: "Garden".into(),
        yours: true,
        state: "waiting".into(),
        seats: vec![
            GameSeat {
                you: true,
                taken: true,
                ..GameSeat::default()
            },
            GameSeat::default(),
        ],
        ..GameSummary::default()
    }]);
    lobby.apply(LobbyEvent::Games(listing.clone()));
    assert_eq!(lobby.query().q, "room");
    assert_eq!(lobby.query().offset, 0);
    assert!(!lobby.room_dirty());
    lobby.adjust_room(Adjustment::Mulligans(true));
    lobby.set_field(Field::RoomName, "A quiet evening");
    lobby.builder_mut().set_pool(
        vec![
            crate::deckbuilder::PoolCard {
                name: "Forest".into(),
                english_name: "Forest".into(),
                kinds: vec!["Land".into()],
                ..crate::deckbuilder::PoolCard::default()
            },
            crate::deckbuilder::PoolCard {
                name: "Island".into(),
                english_name: "Island".into(),
                kinds: vec!["Land".into()],
                ..crate::deckbuilder::PoolCard::default()
            },
        ],
        false,
    );
    lobby.set_field(Field::RoomBoard(0), "forest");
    lobby.room_add_card(0, 0);
    lobby.set_field(Field::RoomBoard(0), "island");
    lobby.room_add_card(0, 1);
    lobby.set_field(Field::RoomCounter, "charge");
    lobby.room_add_counter(0, 0);
    lobby.room_counter_step(0, 0, 0, 2);
    assert!(lobby.field(Field::RoomBoard(0)).is_empty());
    lobby.apply(LobbyEvent::Games(listing));
    assert!(lobby.room_dirty());
    let Some(LobbyRequest::ConfigureRoom { update, .. }) = lobby.save_room(false) else {
        panic!("host edit")
    };
    assert_eq!(update.name, "A quiet evening");
    assert_eq!(update.setup.free_mulligans, 2);
    assert_eq!(update.setup.seats[0].permanents, ["Forest", "Island"]);
    assert_eq!(update.password, None, "an existing lock is preserved");
    assert_eq!(update.setup.seats[0].counters[0][0].amount, 3);
    assert!(update.setup.seats[0].counters[1].is_empty());
}

#[test]
fn room_printing_and_counter_edits_stay_with_their_copy_without_changing_a_deck() {
    let mut lobby = seated_lobby();
    lobby.host(GameMode::Open);
    lobby.apply(LobbyEvent::Seated(SeatHandover {
        game_id: "room".into(),
        seat: 0,
        seat_token: "ticket".into(),
        local: false,
    }));
    lobby.apply(LobbyEvent::Games(GameListing::of(vec![GameSummary {
        id: "room".into(),
        yours: true,
        state: "waiting".into(),
        seats: vec![GameSeat::default(); 2],
        ..GameSummary::default()
    }])));
    lobby.builder_mut().set_pool(
        vec![crate::deckbuilder::PoolCard {
            index: 1,
            name: "Forest".into(),
            english_name: "Forest".into(),
            kinds: vec!["Land".into()],
            scryfall_id: "e04da0ed-d24b-53b8-8f02-7e0d4df382d0".into(),
            ..crate::deckbuilder::PoolCard::default()
        }],
        false,
    );
    for _ in 0..2 {
        lobby.set_field(Field::RoomBoard(0), "forest");
        lobby.room_add_card(0, 0);
    }
    lobby.set_field(Field::RoomCounter, "+2/+1");
    lobby.room_add_counter(0, 1);
    lobby.room_counter_step(0, 1, 0, 4);
    lobby.room_pick_print(0, 1);
    lobby.builder_mut().set_printings(
        1,
        vec![crate::deckbuilder::Printing {
            scryfall_id: "99fbd104-696d-5639-b71a-7275bbd45881".into(),
            set: "m21".into(),
            collector_number: "274".into(),
            lang: "en".into(),
            finishes: vec!["nonfoil".into(), "foil".into()],
            ..crate::deckbuilder::Printing::default()
        }],
        true,
    );
    lobby
        .builder_mut()
        .picker_set_finish(baylee_core::preset::Finish::Foil);
    assert!(lobby.room_confirm_print());
    assert!(
        lobby
            .builder()
            .entries(crate::deckbuilder::Zone::Main)
            .is_empty()
    );
    assert_eq!(
        lobby.room_card(0, 1).unwrap().print.finish_or_default(),
        baylee_core::preset::Finish::Foil
    );
    lobby.room_remove_card(0, 0);
    assert_eq!(
        lobby.room_draft().unwrap().setup.seats[0].counters[0][0].amount,
        5
    );
    assert_eq!(
        lobby.room_card(0, 0).unwrap().print.finish_or_default(),
        baylee_core::preset::Finish::Foil
    );
    lobby.room_counter_step(0, 0, 0, -999);
    assert!(lobby.room_draft().unwrap().setup.seats[0].counters[0].is_empty());
}
