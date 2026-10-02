//! The table screen: the listing, the chairs in a room, and taking one. A table that is full offers no join, one this seat is waiting at is announced and never sat at, a rematch room asks to be played rather than readied, and a listing still in flight when the seat was granted must not dial a second time. Offline is the same screen with this process answering the requests, so it is pressed through here as well — from the offline button to a room of four, a chair moved onto a side, ready, start, and a host installed at the end of it — together with the three gateway controls it must not draw, since a search, a refresh and a room password are all questions about other people. Leaving the game again is the end screen's part.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

#[test]
fn an_empty_search_has_a_working_way_back_to_all_tables() {
    let mut app = headless();
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.apply(LobbyEvent::LoggedIn {
            token: "tok".into(),
            username: None,
        });
        state.lobby.apply(LobbyEvent::Decks(Vec::new()));
        state.lobby.apply(LobbyEvent::Games(GameListing::default()));
        state.lobby.set_field(Field::Search, "an absent table");
    }
    app.update();
    assert!(
        presses(&mut app).contains(&Press::OpenRoom(2)),
        "room creation works before choosing a deck"
    );
    tap_control(&mut app, "clear the table search", |p| {
        *p == Press::ClearSearch
    });
    assert_eq!(
        app.world()
            .resource::<LobbyState>()
            .lobby
            .field(Field::Search),
        ""
    );
    // The fresh response restores the first-deck invitation, not an empty
    // search and not an enabled action that would try to host without a deck.
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .apply(LobbyEvent::Games(GameListing::default()));
    app.update();
    assert!(labels(&mut app).contains(&Phrase::EmptyTablesTitle.text(Lang::En).to_owned()));
    assert!(presses(&mut app).contains(&Press::BrowseHouse));
    assert!(presses(&mut app).contains(&Press::NewDeck));
    assert!(!presses(&mut app).contains(&Press::ClearSearch));
}

#[test]
fn the_table_screen_builds_once_there_is_a_deck() {
    let mut app = headless();
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.apply(LobbyEvent::LoggedIn {
            token: "tok".to_string(),
            username: None,
        });
        state.lobby.apply(LobbyEvent::Decks(vec![DeckSummary {
            id: "d1".to_string(),
            name: "Allytifact".to_string(),
            cards: 96,
            copies: 100,
            sideboard: 2,
            side_copies: 4,
            commanders: Vec::new(),
            ..Default::default()
        }]));
        state
            .lobby
            .apply(LobbyEvent::Games(GameListing::of(vec![GameSummary {
                id: "0123456789abcdef".to_string(),
                state: "waiting".to_string(),
                seats: vec![
                    GameSeat {
                        seat: 0,
                        taken: true,
                        ..GameSeat::default()
                    },
                    GameSeat {
                        seat: 1,
                        taken: false,
                        ..GameSeat::default()
                    },
                ],
                ..GameSummary::default()
            }])));
    }
    app.update();
    assert!(labels(&mut app).contains(&Phrase::LibraryCounts.fill(Lang::En, &["100", "4"])));
    let found = presses(&mut app);
    for wanted in [
        Press::SignOut,
        Press::Refresh,
        Press::BrowseHouse,
        Press::SelectDeck(0),
        Press::Host(GameMode::Ai),
        Press::OpenRoom(2),
        Press::Join(0),
        // The chairs of a waiting table are drawn for everyone, so a
        // player can take the one they want rather than whichever the
        // gateway would have handed them.
    ] {
        assert!(found.contains(&wanted), "{wanted:?} missing from {found:?}");
    }
}

/// Offline play, pressed all the way through to a table.
///
/// Every decision in this sequence has a test of its own — the lobby's
/// screens, the performer's room, the preset the start button builds — and
/// not one of them says the buttons are wired to any of it. This is the path
/// a player's finger takes: the same `clicks` system, the same mailbox a
/// gateway's answers would arrive through, and at the end a host actually
/// installed for a table of four.
#[test]
fn offline_play_can_be_pressed_all_the_way_to_a_table() {
    let mut app = headless();
    app.world_mut().resource_mut::<LobbyState>().offline =
        Some(super::offline::Offline::without_a_file());

    to_gateway_face(&mut app);
    tap_control(&mut app, "play offline", |p| *p == Press::PlayOffline);
    assert_eq!(
        *app.world().resource::<LobbyState>().lobby.screen(),
        Screen::Table,
        "offline play opens the table screen, not a duel"
    );

    tap_control(&mut app, "open room", |p| *p == Press::OpenRoom(2));
    press(
        &mut app,
        Press::RoomAdjust(client_core::lobby::room::Adjustment::Chairs(true)),
    );
    press(
        &mut app,
        Press::RoomAdjust(client_core::lobby::room::Adjustment::Chairs(true)),
    );
    tap_control(&mut app, "apply settings", |p| *p == Press::SaveRoom(false));
    {
        let state = app.world().resource::<LobbyState>();
        let room = state.lobby.games().first().expect("the room is listed");
        assert_eq!(room.seats.len(), 4);
        assert!(!state.connected, "nothing has started yet");
    }

    // A chair moved onto a side, to prove the room's own controls reach the
    // performer and not only the two buttons that open and close it.
    tap_control(&mut app, "chair one onto a side", |p| {
        matches!(p, Press::SeatTeam(0, 0, _))
    });
    assert_eq!(
        app.world().resource::<LobbyState>().lobby.games()[0].seats[0].team,
        Some(1)
    );

    assert!(
        !presses(&mut app)
            .iter()
            .any(|p| matches!(p, Press::Ready(..))),
        "the host starts directly"
    );
    tap_control(&mut app, "the start button", |p| *p == Press::StartRoom(0));

    let state = app.world().resource::<LobbyState>();
    let Screen::Seated(handover) = state.lobby.screen() else {
        panic!("the start button seats you: {:?}", state.lobby.screen())
    };
    assert!(handover.local, "and the game it seats you at runs here");
    assert!(state.connected, "with a host installed for it");
    assert!(app.world().contains_resource::<InstalledHost>());
}

#[test]
fn room_tab_visits_only_expanded_inputs_in_both_directions() {
    let mut app = headless();
    app.world_mut().resource_mut::<LobbyState>().offline =
        Some(super::offline::Offline::without_a_file());
    to_gateway_face(&mut app);
    tap_control(&mut app, "play offline", |p| *p == Press::PlayOffline);
    tap_control(&mut app, "open room", |p| *p == Press::OpenRoom(2));
    let tab = |app: &mut App, backwards: bool| {
        if backwards {
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::ShiftLeft);
        }
        app.world_mut()
            .resource_mut::<Messages<KeyboardInput>>()
            .write(pressed(KeyCode::Tab, Key::Tab));
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::ShiftLeft);
        app.world().resource::<LobbyState>().lobby.focus()
    };
    assert_eq!(tab(&mut app, false), Field::RoomName);
    assert_eq!(tab(&mut app, true), Field::RoomName);
    tap_control(&mut app, "expand seat two", |p| *p == Press::RoomSetup(1));
    assert_eq!(tab(&mut app, false), Field::RoomBoard(1));
    assert_eq!(tab(&mut app, true), Field::RoomName);
    press(
        &mut app,
        Press::RoomAdjust(client_core::lobby::room::Adjustment::Template(2)),
    );
    tap_control(&mut app, "edit a starting card", |p| {
        *p == Press::RoomCardEdit(1, 0)
    });
    assert_eq!(tab(&mut app, false), Field::RoomBoard(1));
    assert_eq!(tab(&mut app, false), Field::RoomCounter);
    assert_eq!(tab(&mut app, false), Field::RoomName);
    assert_eq!(tab(&mut app, true), Field::RoomCounter);
    tap_control(&mut app, "collapse the editor", |p| {
        *p == Press::RoomSetup(1)
    });
    assert_eq!(tab(&mut app, true), Field::RoomName);
}

#[test]
fn starting_cards_expand_per_seat_without_losing_the_draft_or_hidden_focus() {
    let mut app = headless();
    app.world_mut().resource_mut::<LobbyState>().offline =
        Some(super::offline::Offline::without_a_file());
    to_gateway_face(&mut app);
    tap_control(&mut app, "play offline", |p| *p == Press::PlayOffline);
    tap_control(&mut app, "open room", |p| *p == Press::OpenRoom(2));
    assert!(!presses(&mut app).contains(&Press::Focus(Field::RoomBoard(0))));
    // A nonempty preset stays visible as a count even while its editor is shut.
    press(
        &mut app,
        Press::RoomAdjust(client_core::lobby::room::Adjustment::Template(2)),
    );
    assert!(
        labels(&mut app)
            .iter()
            .any(|s| s.contains("Starting cards · 5"))
    );
    tap_control(&mut app, "expand starting cards", |p| {
        *p == Press::RoomSetup(0)
    });
    tap_control(&mut app, "focus card search", |p| {
        *p == Press::Focus(Field::RoomBoard(0))
    });
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .set_field(Field::RoomBoard(0), "island");
    tap_control(&mut app, "expand the other seat", |p| {
        *p == Press::RoomSetup(1)
    });
    let controls = presses(&mut app);
    assert!(!controls.contains(&Press::Focus(Field::RoomBoard(0))));
    assert!(controls.contains(&Press::Focus(Field::RoomBoard(1))));
    let state = app.world().resource::<LobbyState>();
    assert_eq!(state.lobby.focus(), Field::RoomName);
    assert_eq!(state.lobby.field(Field::RoomBoard(0)), "island");
    assert_eq!(
        state.lobby.room_draft().unwrap().setup.seats[0]
            .permanents
            .len(),
        5
    );
    tap_control(&mut app, "collapse starting cards", |p| {
        *p == Press::RoomSetup(1)
    });
    assert!(!presses(&mut app).contains(&Press::Focus(Field::RoomBoard(1))));
}

/// And the one-tap duel is one tap.
///
/// `mode: "ai"` seats you at once — the lobby takes that handover straight
/// to the table without waiting for anybody — so a performer that opened a
/// room and started nothing would hand over a seat with no game behind it,
/// and this button would fail every single press.
#[test]
fn playing_the_house_offline_is_still_one_press() {
    let mut app = headless();
    app.world_mut().resource_mut::<LobbyState>().offline =
        Some(super::offline::Offline::without_a_file());

    to_gateway_face(&mut app);
    tap_control(&mut app, "play offline", |p| *p == Press::PlayOffline);
    let words = labels(&mut app);
    assert!(words.contains(&Phrase::OfflineReadyToPlay.text(Lang::En).to_string()));
    assert!(!words.contains(&Phrase::NoTablesOpen.text(Lang::En).to_string()));
    let mut controls = app.world_mut().query::<&Press>();
    assert_eq!(
        controls
            .iter(app.world())
            .filter(|p| **p == Press::Host(GameMode::Ai))
            .count(),
        1
    );
    let mut lists = app.world_mut().query::<&Scrollable>();
    assert!(!lists.iter(app.world()).any(|list| list.0 == List::Games));
    tap_control(&mut app, "play the house", |p| {
        *p == Press::Host(GameMode::Ai)
    });

    let state = app.world().resource::<LobbyState>();
    assert!(
        matches!(state.lobby.screen(), Screen::Seated(h) if h.local),
        "one press seats you: {:?}",
        state.lobby.screen()
    );
    assert!(state.connected, "with a host installed for it");
}

/// Offline, the screen asks nothing that only a gateway could answer.
///
/// A search over open tables, the refresh beside it and a room password are
/// three questions about other people, and offline there are none: the only
/// table that can exist is the one this process is holding, and it is on the
/// screen already. They were drawn anyway, and pressing either button sent a
/// request the performer answered with "that needs a gateway" — a control
/// whose whole behaviour is a refusal.
#[test]
fn the_offline_lobby_draws_none_of_the_gateways_controls() {
    let mut app = headless();
    app.world_mut().resource_mut::<LobbyState>().offline =
        Some(super::offline::Offline::without_a_file());

    to_gateway_face(&mut app);
    tap_control(&mut app, "play offline", |p| *p == Press::PlayOffline);
    let presses: Vec<Press> = {
        let mut query = app.world_mut().query::<&Press>();
        query.iter(app.world()).copied().collect()
    };
    assert!(
        presses.contains(&Press::OpenRoom(2)),
        "the table's own controls stay: {presses:?}"
    );
    assert!(
        !presses.contains(&Press::Search),
        "nothing to search: {presses:?}"
    );
    assert!(
        !presses.contains(&Press::Refresh),
        "nothing to refresh: {presses:?}"
    );
    let words = labels(&mut app);
    assert!(
        !words.iter().any(|l| l.contains("ROOM PASSWORD")),
        "and no lock on a room nobody else can reach: {words:?}"
    );
}

#[test]
fn a_table_we_are_waiting_at_is_announced_and_not_sat_at() {
    let mut app = headless();
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.apply(LobbyEvent::LoggedIn {
            token: "tok".to_string(),
            username: None,
        });
        state.lobby.apply(LobbyEvent::Decks(vec![DeckSummary {
            id: "d1".to_string(),
            name: "Allytifact".to_string(),
            cards: 96,
            sideboard: 0,
            commanders: Vec::new(),
            ..Default::default()
        }]));
        state.lobby.apply(LobbyEvent::Games(GameListing::default()));
        state.lobby.host(GameMode::Open);
        state.lobby.apply(LobbyEvent::Seated(SeatHandover {
            game_id: "0123456789".to_string(),
            seat: 0,
            seat_token: "st".to_string(),
            local: false,
        }));
    }
    app.update();
    assert!(
        labels(&mut app)
            .iter()
            .any(|l| l.contains("waiting for an opponent")),
        "the open table is on screen"
    );
    // And no duel was opened: a socket here would be closed straight back.
    assert!(app.world().get_resource::<InstalledHost>().is_none());
}

#[test]
fn a_reply_that_lands_after_the_seat_was_taken_does_not_dial_again() {
    let mut app = headless();
    stocked(&mut app);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        // Against the house, which is the one seat that is playable the
        // moment it is granted: `mode:"ai"` orders an engine before the
        // gateway answers. A chair at a *room* is held rather than played,
        // so asking for one of those would set up a different situation
        // altogether — the seat screen would still be waiting.
        state.lobby.host(GameMode::Ai);
        state.lobby.apply(LobbyEvent::Seated(SeatHandover {
            game_id: "g1".to_string(),
            seat: 0,
            seat_token: "st".to_string(),
            local: false,
        }));
        // Stand in for a dial that already succeeded.
        state.connected = true;
    }
    // A `ListGames` that was already in flight when the seat was granted.
    app.world()
        .resource::<Mailbox>()
        .0
        .lock()
        .expect("mailbox")
        .push(Reply::Event(LobbyEvent::Games(GameListing::default())));
    app.update();
    assert!(
        matches!(
            app.world().resource::<LobbyState>().lobby.screen(),
            Screen::Seated(_)
        ),
        "a second dial would have failed and unseated us"
    );
}

/// The other end of the same press. A room opened by somebody else's rematch
/// is a waiting table with this player's chair in it, and the only thing that
/// distinguishes it is the flag the gateway sets — without which the row
/// would offer *ready*, be answered `200`, and change nothing at all.
#[test]
fn a_rematch_room_in_the_list_asks_to_be_played_rather_than_readied() {
    let mut app = headless();
    stocked(&mut app);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state
            .lobby
            .apply(LobbyEvent::Games(GameListing::of(vec![GameSummary {
                id: "g2".to_string(),
                name: "Kitchen".to_string(),
                state: "waiting".to_string(),
                rematch: true,
                seats: vec![
                    GameSeat {
                        seat: 0,
                        taken: true,
                        you: true,
                        ready: false,
                        ..GameSeat::default()
                    },
                    GameSeat {
                        seat: 1,
                        taken: true,
                        ready: true,
                        ..GameSeat::default()
                    },
                ],
                ..GameSummary::default()
            }])));
    }
    app.update();
    let found = presses(&mut app);
    assert!(found.contains(&Press::Rematch(0)), "{found:?}");
    assert!(
        !found.contains(&Press::Ready(0, true)),
        "ready cannot claim a reserved chair: {found:?}"
    );
}

#[test]
fn a_table_that_is_full_offers_no_join() {
    let mut app = headless();
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.apply(LobbyEvent::LoggedIn {
            token: "tok".to_string(),
            username: None,
        });
        state
            .lobby
            .apply(LobbyEvent::Games(GameListing::of(vec![GameSummary {
                id: "g".to_string(),
                state: "playing".to_string(),
                seats: vec![
                    GameSeat {
                        seat: 0,
                        taken: true,
                        ..GameSeat::default()
                    },
                    GameSeat {
                        seat: 1,
                        taken: true,
                        ..GameSeat::default()
                    },
                ],
                ..GameSummary::default()
            }])));
    }
    app.update();
    assert!(!presses(&mut app).contains(&Press::Join(0)));
}
