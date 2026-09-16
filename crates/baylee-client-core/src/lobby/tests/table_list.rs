//! The listing read as data and walked as pages. What a `GameSummary` says about itself is settled here — joinable only while a chair is free for a *person*, a headline that counts who is sitting down rather than who is ready, a name where the table has none, and which chair is mine coming from the gateway saying so and never from comparing account ids — together with the gateway's own JSON decoding into these DTOs at all. The pager is the other half: it asks only for what it does not hold, a search reads the list from the top because it is a different list, and a page that has emptied underneath a player falls back to the first. Acting on a row is `rooms`, and the seat a row earns is `seating`.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

/// A full table is not a ready one, and the line under a room's name has
/// to say which it is. It counted ready chairs back when having a deck
/// *was* being ready, and would have read "0/4 seated" at a full table
/// the moment that stopped being true.
#[test]
fn a_rooms_headline_counts_who_is_sitting_down_not_who_is_ready() {
    let room = GameSummary {
        id: "g".to_string(),
        name: "Kitchen".to_string(),
        state: "waiting".to_string(),
        seats: vec![
            GameSeat {
                seat: 0,
                taken: true,
                ready: false,
                ..GameSeat::default()
            },
            GameSeat {
                seat: 1,
                kind: SeatKind::Ai,
                ready: true,
                ..GameSeat::default()
            },
            GameSeat {
                seat: 2,
                ..GameSeat::default()
            },
        ],
        ..GameSummary::default()
    };
    assert!(room.headline().contains("2/3"), "{}", room.headline());
    assert!(!room.i_am_ready(), "nobody here is this player");
}

#[test]
fn only_a_waiting_table_with_a_free_seat_is_joinable() {
    let waiting = GameSummary {
        id: "g".to_string(),
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
    };
    assert!(waiting.joinable());
    let playing = GameSummary {
        state: "playing".to_string(),
        ..waiting.clone()
    };
    assert!(!playing.joinable());
    let full = GameSummary {
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
        ..waiting
    };
    assert!(!full.joinable());
}

#[test]
fn the_gateways_own_json_decodes() {
    let decks: Vec<DeckSummary> =
        serde_json::from_str(r#"[{"id":"d1","name":"Allytifact","cards":60,"commanders":[]}]"#)
            .expect("deck list");
    assert_eq!(decks[0].name, "Allytifact");
    let listing: GameListing = serde_json::from_str(
        r#"{"games":[{"id":"g1","state":"waiting","seats":[{"seat":0,"taken":true},{"seat":1,"taken":false}]}],"total":9,"offset":8,"limit":8}"#,
    )
    .expect("game list");
    assert!(listing.games[0].joinable());
    assert_eq!((listing.total, listing.offset), (9, 8));
    let seat: SeatHandover =
        serde_json::from_str(r#"{"game_id":"g1","seat":0,"seat_token":"tok"}"#).expect("handover");
    assert_eq!(seat.seat_token, "tok");
}

/// The pager asks for what it does not have, and walks off neither end.
#[test]
fn the_table_list_is_paged_in_both_directions() {
    let mut lobby = seated_lobby();
    assert_eq!(lobby.page(true), None, "no second page of an empty lobby");
    assert_eq!(lobby.page(false), None, "and nothing before the first");

    lobby.apply(LobbyEvent::Games(GameListing {
        games: a_page(PAGE),
        total: PAGE + 3,
        offset: 0,
        limit: PAGE,
    }));
    assert!(lobby.more(), "three tables did not fit");
    assert_eq!(lobby.page(false), None, "the first page is the first page");
    assert_eq!(
        lobby.page(true),
        Some(LobbyRequest::ListGames(GameQuery {
            q: String::new(),
            offset: PAGE,
            limit: PAGE,
        }))
    );

    lobby.apply(LobbyEvent::Games(GameListing {
        games: a_page(3),
        total: PAGE + 3,
        offset: PAGE,
        limit: PAGE,
    }));
    assert!(!lobby.more(), "that was the end of the list");
    assert_eq!(lobby.page(true), None);
    assert_eq!(
        lobby.page(false),
        Some(LobbyRequest::ListGames(GameQuery {
            q: String::new(),
            offset: 0,
            limit: PAGE,
        }))
    );
}

/// A search is a different list, so it is read from the top — the row
/// that was ninth in the old one is not the ninth in this one.
#[test]
fn searching_starts_the_list_again() {
    let mut lobby = seated_lobby();
    lobby.apply(LobbyEvent::Games(GameListing {
        games: a_page(PAGE),
        total: PAGE + 1,
        offset: 0,
        limit: PAGE,
    }));
    lobby.page(true);
    lobby.apply(LobbyEvent::Games(GameListing {
        games: a_page(1),
        total: PAGE + 1,
        offset: PAGE,
        limit: PAGE,
    }));
    assert_eq!(lobby.offset(), PAGE);

    lobby.set_field(Field::Search, "kitchen");
    assert_eq!(
        lobby.search_again(),
        Some(LobbyRequest::ListGames(GameQuery {
            q: "kitchen".to_string(),
            offset: 0,
            limit: PAGE,
        }))
    );
    assert_eq!(lobby.offset(), 0);
}

/// The last table on page two closing must not leave a player looking at
/// an empty page two.
#[test]
fn a_page_that_no_longer_exists_falls_back_to_the_first() {
    let mut lobby = seated_lobby();
    lobby.apply(LobbyEvent::Games(GameListing {
        games: a_page(PAGE),
        total: PAGE + 1,
        offset: 0,
        limit: PAGE,
    }));
    lobby.page(true);
    let next = lobby.apply(LobbyEvent::Games(GameListing {
        games: Vec::new(),
        total: 4,
        offset: PAGE,
        limit: PAGE,
    }));
    assert_eq!(
        next,
        Some(LobbyRequest::ListGames(GameQuery {
            q: String::new(),
            offset: 0,
            limit: PAGE,
        }))
    );
    assert_eq!(lobby.offset(), 0);
}

/// A table with one seat left is joinable; the same table with that seat
/// handed to the AI is not, because there is no chair for a person.
#[test]
fn a_table_is_joinable_only_while_a_chair_is_free_for_a_person() {
    let mut room = GameSummary {
        id: "g".to_string(),
        name: "Kitchen table".to_string(),
        host: Some("ada".to_string()),
        yours: false,
        state: "waiting".to_string(),
        seats: vec![
            GameSeat {
                seat: 0,
                taken: true,
                player: Some("ada".to_string()),
                ready: true,
                ..GameSeat::default()
            },
            GameSeat {
                seat: 1,
                ..GameSeat::default()
            },
        ],
        ..GameSummary::default()
    };
    assert!(room.joinable());
    room.seats[1].kind = SeatKind::Ai;
    room.seats[1].ai = Some("sharp".to_string());
    room.seats[1].ready = true;
    assert!(!room.joinable(), "the AI has that chair");
    assert_eq!(room.headline(), "Kitchen table  \u{b7}  2/2 seated");

    // A table already playing is never joinable, free chair or not.
    room.seats[1].kind = SeatKind::Human;
    room.seats[1].taken = false;
    room.state = "playing".to_string();
    assert!(!room.joinable());
}

/// Which chair is mine comes from the gateway saying so, not from the
/// client comparing account ids it should not have.
#[test]
fn the_gateway_says_which_chair_is_mine() {
    let room = GameSummary {
        id: "g".to_string(),
        state: "waiting".to_string(),
        seats: vec![
            GameSeat {
                seat: 0,
                taken: true,
                player: Some("ada".to_string()),
                ..GameSeat::default()
            },
            GameSeat {
                seat: 1,
                taken: true,
                you: true,
                player: Some("grace".to_string()),
                ..GameSeat::default()
            },
        ],
        ..GameSummary::default()
    };
    assert_eq!(room.my_seat(), Some(1));
    assert!(room.seated());
    assert!(!GameSummary::default().seated());
}

/// A table with no name still reads as something in the list.
#[test]
fn a_nameless_table_still_has_a_headline() {
    let room = GameSummary {
        state: "waiting".to_string(),
        seats: vec![GameSeat::default(), GameSeat::default()],
        ..GameSummary::default()
    };
    assert_eq!(room.headline(), "table  \u{b7}  0/2 seated");
}
