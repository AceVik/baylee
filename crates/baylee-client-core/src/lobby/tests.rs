use super::*;
use crate::deckbuilder::{Coverage, PoolCard, Zone};

/// A signed-in lobby with one deck, without walking the whole flow.
fn seated_lobby() -> Lobby {
    let mut lobby = Lobby::new();
    lobby.set_field(Field::Email, "a@b.c");
    lobby.set_field(Field::Password, "hunter22");
    assert!(lobby.submit().is_some());
    assert_eq!(
        lobby.apply(LobbyEvent::LoggedIn {
            token: "tok".to_string()
        }),
        Some(LobbyRequest::ListDecks)
    );
    assert_eq!(
        lobby.apply(LobbyEvent::Decks(vec![DeckSummary {
            sideboard: 0,
            id: "d1".to_string(),
            name: "Allytifact".to_string(),
            cards: 60,
            commanders: Vec::new(),
        }])),
        Some(LobbyRequest::ListGames(lobby.query()))
    );
    lobby.apply(LobbyEvent::Games(GameListing::default()));
    lobby
}

/// The same lobby offline: no account, one deck, the tables listed.
fn offline_lobby() -> Lobby {
    let mut lobby = Lobby::new();
    assert_eq!(lobby.play_offline(), Some(LobbyRequest::ListDecks));
    assert_eq!(
        lobby.apply(LobbyEvent::Decks(vec![DeckSummary {
            sideboard: 0,
            id: "d1".to_string(),
            name: "Allytifact".to_string(),
            cards: 60,
            commanders: Vec::new(),
        }])),
        Some(LobbyRequest::ListGames(lobby.query()))
    );
    lobby.apply(LobbyEvent::Games(GameListing::default()));
    lobby
}

/// Offline reaches the builder, with no account behind it.
///
/// Every intent method used to read the token as its proof that a request
/// could go anywhere at all, and offline play holds none — so the builder,
/// the room and the start button were one dead press each, on a screen
/// that went on drawing all three.
#[test]
fn offline_play_reaches_the_builder() {
    let mut lobby = offline_lobby();
    assert_eq!(lobby.token(), None, "there is no account behind this");
    assert!(lobby.build_deck().is_some(), "and it asks for the pool");
    assert_eq!(*lobby.screen(), Screen::Build);
}

/// And it reaches a room, which is the other half of the same rule.
#[test]
fn offline_play_reaches_a_room() {
    let mut lobby = offline_lobby();
    assert!(matches!(
        lobby.open_room(GameMode::Open, 4, "Kitchen".to_string()),
        Some(LobbyRequest::CreateGame { chairs: 4, .. })
    ));
}

/// An offline table is not waiting for anybody who could arrive.
///
/// It is the same event as the gateway's open table and a different
/// fact: every other chair here is the house already, so a note reading
/// "waiting for an opponent" is a sentence about a player who cannot
/// come. What this table waits for is the person at the keyboard.
#[test]
fn an_offline_table_says_what_it_is_actually_waiting_for() {
    let mut lobby = offline_lobby();
    assert!(lobby.offline(), "this is the performer we came in with");
    lobby.open_room(GameMode::Open, 4, "Kitchen".to_string());
    lobby.apply(LobbyEvent::Seated(SeatHandover {
        game_id: "offline".to_string(),
        seat: 0,
        seat_token: "st".to_string(),
        local: true,
    }));
    assert_eq!(lobby.status(), "table open — arrange the chairs and start");
}

/// Offline asks for no chair back, however the listing reads.
///
/// `reclaim_a_seat` recovers a ticket that died with the process while
/// the *table* lived on at a gateway. Offline the table is the process,
/// so there is nobody to ask — and the offline performer refuses the
/// request in words, which came out as a red line in the corner of the
/// lobby of a player who had just finished a game against the house.
/// The counter-test is
/// `a_restarted_client_asks_for_the_chair_it_is_still_sitting_in`: the
/// same listing at a gateway is worth a ticket.
#[test]
fn offline_asks_for_no_chair_it_could_never_be_given() {
    let mut lobby = offline_lobby();
    let mine = GameListing::of(vec![GameSummary {
        id: "offline".to_string(),
        state: "playing".to_string(),
        seats: vec![GameSeat {
            seat: 0,
            taken: true,
            you: true,
            ..GameSeat::default()
        }],
        ..GameSummary::default()
    }]);
    assert_eq!(
        lobby.apply(LobbyEvent::Games(mine)),
        None,
        "offline has nothing to hand a chair back"
    );
    assert!(!lobby.busy(), "and nothing is in flight for it");
}

/// And the gateway's own table keeps the sentence it had.
#[test]
fn a_gateways_open_table_is_still_waiting_for_an_opponent() {
    let mut lobby = seated_lobby();
    assert!(!lobby.offline(), "there is an account behind this one");
    lobby.host(GameMode::Open);
    lobby.apply(LobbyEvent::Seated(SeatHandover {
        game_id: "g1".to_string(),
        seat: 0,
        seat_token: "st".to_string(),
        local: false,
    }));
    assert_eq!(lobby.status(), "table open — waiting for an opponent");
}

/// Signing out of offline play puts the sign-in screen back in charge.
#[test]
fn leaving_offline_play_takes_the_performer_with_it() {
    let mut lobby = offline_lobby();
    lobby.sign_out();
    assert_eq!(lobby.refresh(), None, "there is nobody to ask again");
}

#[test]
fn a_sign_in_needs_both_fields() {
    let mut lobby = Lobby::new();
    assert_eq!(lobby.submit(), None);
    assert!(!lobby.busy());
    lobby.type_char('a');
    assert_eq!(lobby.submit(), None, "a password is still missing");
    lobby.focus_on(Field::Password);
    lobby.type_char('x');
    assert!(matches!(lobby.submit(), Some(LobbyRequest::LogIn { .. })));
}

#[test]
fn registering_also_needs_a_display_name() {
    let mut lobby = Lobby::new();
    lobby.toggle_registering();
    lobby.focus_on(Field::Email);
    lobby.type_char('a');
    lobby.focus_on(Field::Password);
    lobby.type_char('x');
    assert_eq!(lobby.submit(), None);
    lobby.focus_on(Field::DisplayName);
    lobby.type_char('V');
    assert!(matches!(
        lobby.submit(),
        Some(LobbyRequest::Register { .. })
    ));
}

#[test]
fn a_sign_up_chains_into_a_log_in() {
    let mut lobby = Lobby::new();
    lobby.toggle_registering();
    lobby.type_char('a');
    lobby.focus_on(Field::DisplayName);
    lobby.type_char('V');
    lobby.focus_on(Field::Password);
    lobby.type_char('x');
    lobby.submit();
    assert_eq!(
        lobby.apply(LobbyEvent::Registered {
            confirmation_required: false,
        }),
        Some(LobbyRequest::LogIn {
            email: "a".to_string(),
            password: "x".to_string(),
        }),
        "the gateway hands out no token on sign-up"
    );
    assert!(lobby.busy(), "the chained log-in is in flight");
}

#[test]
fn signing_in_asks_for_decks_and_then_for_games() {
    let lobby = seated_lobby();
    assert_eq!(*lobby.screen(), Screen::Table);
    assert_eq!(lobby.token(), Some("tok"));
    assert_eq!(lobby.selected(), Some(0), "the only deck is picked for us");
    assert!(!lobby.busy(), "the chain ended");
}

#[test]
fn the_password_is_dropped_once_it_has_been_spent() {
    let lobby = seated_lobby();
    assert_eq!(lobby.field(Field::Password), "");
}

/// The lobby's own sentences are drawn from the phrase table, so setting
/// the language changes what the *next* one says. Nothing re-translates a
/// line already on screen, which is the point: a status line is a record
/// of what just happened, not a label that keeps re-rendering.
#[test]
fn a_status_line_is_said_in_the_lobbys_language() {
    let mut lobby = seated_lobby();
    lobby.apply(LobbyEvent::Decks(vec![]));
    lobby.apply(LobbyEvent::Games(GameListing::default()));
    assert_eq!(lobby.host(GameMode::Ai), None);
    assert_eq!(lobby.status(), "pick a deck first");

    lobby.set_lang(Lang::De);
    assert_eq!(lobby.lang(), Lang::De);
    assert_eq!(lobby.host(GameMode::Ai), None);
    assert_eq!(lobby.status(), "wähle zuerst ein Deck");
}

/// The shell says things too, and has no business knowing the language.
#[test]
fn the_shell_says_its_own_sentences_in_that_language_too() {
    let mut lobby = seated_lobby();
    lobby.set_lang(Lang::De);
    lobby.tell(Phrase::CouldNotReachTable, &["Zeitüberschreitung"]);
    assert_eq!(lobby.status(), "Tisch nicht erreichbar: Zeitüberschreitung");
}

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
fn a_granted_seat_ends_on_the_seated_screen() {
    let mut lobby = seated_lobby();
    lobby.host(GameMode::Ai);
    let handover = SeatHandover {
        game_id: "g1".to_string(),
        seat: 0,
        seat_token: "st".to_string(),
        local: false,
    };
    assert_eq!(lobby.apply(LobbyEvent::Seated(handover.clone())), None);
    assert_eq!(*lobby.screen(), Screen::Seated(handover));
}

#[test]
fn a_table_we_cannot_reach_hands_the_lobby_back() {
    let mut lobby = seated_lobby();
    lobby.host(GameMode::Ai);
    lobby.apply(LobbyEvent::Seated(SeatHandover {
        game_id: "g1".to_string(),
        seat: 0,
        seat_token: "st".to_string(),
        local: false,
    }));
    lobby.unseat("the table did not answer");
    assert_eq!(*lobby.screen(), Screen::Table);
    assert_eq!(lobby.status(), "the table did not answer");
    assert!(
        lobby.refresh().is_some(),
        "and the lobby takes requests again"
    );
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
fn an_open_table_is_not_sat_at_until_somebody_joins() {
    let mut lobby = seated_lobby();
    lobby.host(GameMode::Open);
    let handover = SeatHandover {
        game_id: "g1".to_string(),
        seat: 0,
        seat_token: "st".to_string(),
        local: false,
    };
    assert_eq!(
        lobby.apply(LobbyEvent::Seated(handover.clone())),
        Some(LobbyRequest::ListGames(lobby.query())),
        "the gateway builds the session on the second seat, not the first"
    );
    assert_eq!(*lobby.screen(), Screen::Table);
    assert_eq!(lobby.awaiting(), Some(&handover));

    // Still only us at the table.
    lobby.apply(LobbyEvent::Games(GameListing::of(vec![GameSummary {
        id: "g1".to_string(),
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
    assert_eq!(*lobby.screen(), Screen::Table);

    lobby.apply(LobbyEvent::Games(GameListing::of(vec![GameSummary {
        id: "g1".to_string(),
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
    assert_eq!(*lobby.screen(), Screen::Seated(handover));
    assert_eq!(lobby.awaiting(), None);
}

/// The other half of that, and the half that was missing: **joining**
/// somebody else's room does not start it either.
///
/// The owner sat down at a room I was hosting and was shown a duel with
/// no game behind it — the sky, two seat mats, nothing else — because
/// the join path handed the seat straight to `Screen::Seated`. A room
/// starts on two statements by two people, and neither of them is
/// sitting down; worse, the duel it opened had no way back to the lobby,
/// so the `ready` it was waiting for could never be given.
#[test]
fn joining_a_room_waits_for_it_to_start_like_opening_one_does() {
    let mut lobby = seated_lobby();
    let handover = SeatHandover {
        game_id: "g1".to_string(),
        seat: 1,
        seat_token: "st".to_string(),
        local: false,
    };
    // No `host()` call: this is a join, which is the path that did not
    // set `asked_for` and therefore took the other branch.
    assert_eq!(
        lobby.apply(LobbyEvent::Seated(handover.clone())),
        Some(LobbyRequest::ListGames(lobby.query())),
        "a seat at a room is held, not played"
    );
    assert_eq!(*lobby.screen(), Screen::Table, "still in the lobby");
    assert_eq!(lobby.awaiting(), Some(&handover));

    // Seated, with a deck, and nobody has said `ready` yet.
    lobby.apply(LobbyEvent::Games(GameListing::of(vec![GameSummary {
        id: "g1".to_string(),
        state: "waiting".to_string(),
        seats: vec![
            GameSeat {
                seat: 0,
                taken: true,
                ready: true,
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
    assert_eq!(
        *lobby.screen(),
        Screen::Table,
        "a full room is still a room until the host starts it"
    );

    lobby.apply(LobbyEvent::Games(GameListing::of(vec![GameSummary {
        id: "g1".to_string(),
        state: "playing".to_string(),
        seats: vec![
            GameSeat {
                seat: 0,
                taken: true,
                ready: true,
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
    assert_eq!(*lobby.screen(), Screen::Seated(handover));
    assert_eq!(lobby.awaiting(), None);
}

/// The owner's client was restarted while they held a chair, and they
/// then watched their own table turn `"playing"` from the lobby with no
/// way into it: the seat token had died with the process, and `join`
/// refuses a table you are already at.
///
/// Nothing on this side remembers a seat across a restart, so the listing
/// is what notices — it is the only thing holding both halves of the
/// question, that the chair is theirs and that we have nothing for it.
#[test]
fn a_restarted_client_asks_for_the_chair_it_is_still_sitting_in() {
    let mut lobby = seated_lobby();
    let mine = |state: &str| {
        GameListing::of(vec![GameSummary {
            id: "g1".to_string(),
            state: state.to_string(),
            seats: vec![
                GameSeat {
                    seat: 0,
                    taken: true,
                    ready: true,
                    ..GameSeat::default()
                },
                GameSeat {
                    seat: 1,
                    taken: true,
                    ready: true,
                    you: true,
                    ..GameSeat::default()
                },
            ],
            ..GameSummary::default()
        }])
    };

    // A fresh lobby, holding no ticket, reading a table it is sitting at.
    assert_eq!(
        lobby.apply(LobbyEvent::Games(mine("playing"))),
        Some(LobbyRequest::TakeSeat {
            game_id: "g1".to_string()
        })
    );
    assert!(lobby.busy(), "and it is a request, not a note");

    // The gateway answers a ticket like any other. This one has a game
    // behind it *now*, so there is nothing to wait for — telling this
    // player to go and press Bereit would be advice about a button that
    // is no longer on the screen.
    let handover = SeatHandover {
        game_id: "g1".to_string(),
        seat: 1,
        seat_token: "fresh".to_string(),
        local: false,
    };
    assert_eq!(lobby.apply(LobbyEvent::Seated(handover.clone())), None);
    assert_eq!(*lobby.screen(), Screen::Seated(handover));
    assert_eq!(lobby.awaiting(), None, "nothing is being waited for");
}

/// The ask fires off a listing and its answer is another listing, so a
/// refusal that came back per page would be re-sent for ever. A rematch
/// room is left out of it entirely: that chair is reserved rather than
/// taken, and only `POST …/rematch` claims one.
#[test]
fn a_chair_is_asked_for_once_however_often_the_lobby_is_read() {
    let mut lobby = seated_lobby();
    let mut mine = GameSummary {
        id: "g1".to_string(),
        state: "waiting".to_string(),
        seats: vec![GameSeat {
            seat: 0,
            taken: true,
            you: true,
            ..GameSeat::default()
        }],
        ..GameSummary::default()
    };
    let listing = |g: &GameSummary| GameListing::of(vec![g.clone()]);

    assert_eq!(
        lobby.apply(LobbyEvent::Games(listing(&mine))),
        Some(LobbyRequest::TakeSeat {
            game_id: "g1".to_string()
        }),
        "a chair held at a room that has not started is worth a ticket too"
    );
    // The gateway refuses — an older one with no such route, say.
    lobby.apply(LobbyEvent::Failed("no such route".to_string()));
    assert_eq!(
        lobby.apply(LobbyEvent::Games(listing(&mine))),
        None,
        "asked once, not once per page"
    );

    // The same chair at a rematch room asks nothing at all.
    let mut lobby = seated_lobby();
    mine.id = "g2".to_string();
    mine.rematch = true;
    assert_eq!(lobby.apply(LobbyEvent::Games(listing(&mine))), None);
}

/// The press to play again is recorded on one side of the unseating and
/// spent on the other, so the one thing it must survive is the unseating
/// — which clears `busy`, the status line and everything else about the
/// seat, and would clear a request already in flight with them.
#[test]
fn the_press_to_play_again_outlives_leaving_the_table() {
    let mut lobby = seated_lobby();
    lobby.apply(LobbyEvent::Seated(SeatHandover {
        game_id: "g1".to_string(),
        seat: 0,
        seat_token: "st".to_string(),
        local: false,
    }));
    lobby.want_rematch("g1");
    lobby.stand_up(Phrase::GameEnded, &[]);
    assert_eq!(
        lobby.take_rematch(),
        Some(LobbyRequest::Rematch {
            game_id: "g1".to_string()
        })
    );
    assert!(lobby.busy(), "and it is a request, not a note");
    assert_eq!(lobby.take_rematch(), None, "spent exactly once");
}

/// A game that ended is not a refusal, and a table that could not be
/// reached is.
///
/// Both leave the seat by the same door, and the words alone cannot tell
/// them apart — `Tone` is the whole of it, and the lobby draws a refusal
/// in red. Every finished duel used to end with its own ending written
/// up there in the colour that means somebody has to do something.
#[test]
fn the_end_of_a_game_is_read_differently_from_a_table_that_refused_one() {
    let mut lobby = seated_lobby();
    lobby.stand_up(Phrase::GameEnded, &[]);
    assert_eq!(*lobby.screen(), Screen::Table, "and still leaves the seat");
    assert_eq!(lobby.tone(), Tone::Note);

    lobby.unseat_because(Phrase::CouldNotReachTable, &["no route"]);
    assert_eq!(lobby.tone(), Tone::Refusal);
}

/// Leaving without pressing it asks for nothing, which is what lets the
/// shell fall through to re-reading the table list.
#[test]
fn leaving_a_finished_game_asks_for_no_rematch_by_itself() {
    let mut lobby = seated_lobby();
    lobby.stand_up(Phrase::GameEnded, &[]);
    assert_eq!(lobby.take_rematch(), None);
    // And a press that outlives the account it was made under is dropped
    // rather than sent, having nothing to sign it with.
    lobby.want_rematch("g1");
    lobby.sign_out();
    assert_eq!(lobby.take_rematch(), None);
}

#[test]
fn a_table_against_the_house_is_playable_at_once() {
    let mut lobby = seated_lobby();
    lobby.host(GameMode::Ai);
    lobby.apply(LobbyEvent::Seated(SeatHandover {
        game_id: "g1".to_string(),
        seat: 0,
        seat_token: "st".to_string(),
        local: false,
    }));
    assert!(matches!(lobby.screen(), Screen::Seated(_)));
    assert_eq!(lobby.awaiting(), None);
}

/// This asserted that a join is playable at once, which is what the
/// owner's empty duel was. Its *other* claim is the one worth keeping and
/// the reason it was written: an open table of ours that went nowhere
/// must not leave a stale `asked_for` behind that changes what the next
/// seat means.
#[test]
fn a_failed_open_table_does_not_colour_the_next_seat() {
    let mut lobby = seated_lobby();
    lobby.host(GameMode::Open);
    lobby.apply(LobbyEvent::Failed("busy".to_string()));
    lobby.join("g7");
    let handover = SeatHandover {
        game_id: "g7".to_string(),
        seat: 1,
        seat_token: "st".to_string(),
        local: false,
    };
    lobby.apply(LobbyEvent::Seated(handover.clone()));
    assert_eq!(
        *lobby.screen(),
        Screen::Table,
        "a room does not start by being sat at"
    );
    assert_eq!(lobby.awaiting(), Some(&handover));
    // And it is the *joined* table being waited for, not the one that
    // failed to open.
    lobby.apply(LobbyEvent::Games(GameListing::of(vec![GameSummary {
        id: "g7".to_string(),
        state: "playing".to_string(),
        ..GameSummary::default()
    }])));
    assert!(matches!(lobby.screen(), Screen::Seated(_)));
}

#[test]
fn signing_out_forgets_a_table_we_were_waiting_at() {
    let mut lobby = seated_lobby();
    lobby.host(GameMode::Open);
    lobby.apply(LobbyEvent::Seated(SeatHandover {
        game_id: "g1".to_string(),
        seat: 0,
        seat_token: "st".to_string(),
        local: false,
    }));
    lobby.sign_out();
    assert_eq!(lobby.awaiting(), None);
}

#[test]
fn placing_the_caret_is_visible_even_when_it_does_not_move() {
    let mut lobby = Lobby::new();
    let start = lobby.focus_epoch();
    lobby.focus_on(Field::Email);
    assert!(
        lobby.focus_epoch() > start,
        "tapping the field you are already in still has to raise a keyboard"
    );
    let again = lobby.focus_epoch();
    lobby.cycle_focus(Tab::Next);
    assert!(lobby.focus_epoch() > again);
    let refused = lobby.focus_epoch();
    lobby.focus_on(Field::DisplayName);
    assert_eq!(
        lobby.focus_epoch(),
        refused,
        "a field that is not on screen is not focused, so nothing happens"
    );
}

#[test]
fn a_field_can_be_replaced_wholesale() {
    let mut lobby = Lobby::new();
    lobby.set_field(Field::Email, "pasted@example.com");
    assert_eq!(lobby.field(Field::Email), "pasted@example.com");
    lobby.set_field(Field::Email, "");
    assert_eq!(lobby.field(Field::Email), "", "clearing works too");
}

#[test]
fn a_refusal_reads_differently_from_a_note() {
    let mut lobby = Lobby::new();
    assert_eq!(lobby.tone(), Tone::Note, "an empty line is no refusal");
    assert!(lobby.submit().is_none());
    assert_eq!(
        lobby.tone(),
        Tone::Refusal,
        "a form with no address in it is a form that was refused"
    );
    lobby.set_field(Field::Email, "a@b.c");
    lobby.set_field(Field::Password, "pw");
    assert!(lobby.submit().is_some());
    assert_eq!(
        lobby.tone(),
        Tone::Note,
        "and signing in is the lobby getting on with it"
    );
    lobby.apply(LobbyEvent::Failed("wrong password".to_string()));
    assert_eq!(
        lobby.tone(),
        Tone::Refusal,
        "the gateway saying no is the plainest refusal there is"
    );
}

#[test]
fn a_password_can_be_shown_and_covered_again() {
    let mut lobby = Lobby::new();
    assert!(!lobby.showing(Field::Password));
    lobby.toggle_reveal(Field::Password);
    assert!(lobby.showing(Field::Password));
    assert_eq!(
        lobby.focus(),
        Field::Password,
        "pressing the eye beside a box is a way of saying that box"
    );
    lobby.toggle_reveal(Field::Password);
    assert!(!lobby.showing(Field::Password));
}

#[test]
fn the_eye_leaves_a_caret_that_is_already_in_the_box_alone() {
    let mut lobby = Lobby::new();
    lobby.focus_on(Field::Password);
    lobby.set_field_at(Field::Password, "hunter2", 3, None);
    let epoch = lobby.focus_epoch();
    lobby.toggle_reveal(Field::Password);
    assert!(lobby.showing(Field::Password));
    assert_eq!(
        lobby.focus_epoch(),
        epoch,
        "the caret was already there, so nothing re-opens a browser's own input"
    );
    assert_eq!(
        lobby.buffer(Field::Password).cursor(),
        3,
        "and the caret stays where the player was typing"
    );
}

#[test]
fn a_shown_password_is_covered_again_by_leaving_it() {
    let mut lobby = Lobby::new();
    lobby.toggle_reveal(Field::Password);
    lobby.focus_on(Field::Email);
    assert!(
        !lobby.showing(Field::Password),
        "the caret left, so the secret is a secret again"
    );
    lobby.toggle_reveal(Field::Password);
    lobby.cycle_focus(Tab::Next);
    assert!(!lobby.showing(Field::Password), "and Tab is leaving too");
    lobby.toggle_reveal(Field::RoomPassword);
    assert!(
        !lobby.showing(Field::Password),
        "one at a time: a room's password is not the account's"
    );
}

#[test]
fn a_field_says_what_kind_of_keyboard_it_wants() {
    let lobby = Lobby::new();
    assert_eq!(lobby.field_kind(Field::Email), FieldKind::Email);
    assert_eq!(lobby.field_kind(Field::DisplayName), FieldKind::Name);
    assert_eq!(lobby.field_kind(Field::Password), FieldKind::Password);
    assert_eq!(
        lobby.field_kind(Field::RoomPassword),
        FieldKind::Secret,
        "a room's password is not the account's and must not autofill as it"
    );
}

#[test]
fn the_password_box_asks_for_a_new_password_on_the_sign_up_form() {
    let mut lobby = Lobby::new();
    assert_eq!(lobby.field_kind(Field::Password), FieldKind::Password);
    let before = lobby.focus_epoch();
    lobby.toggle_registering();
    assert_eq!(
        lobby.field_kind(Field::Password),
        FieldKind::NewPassword,
        "the same box, and the opposite request to a password manager"
    );
    lobby.focus_on(Field::Password);
    let placed = lobby.focus_epoch();
    lobby.toggle_registering();
    assert!(
        lobby.focus_epoch() > placed,
        "flipping the form under the caret has to re-point the platform's input"
    );
    assert!(before < placed, "the premise: placing the caret counts");
}

#[test]
fn shift_tab_walks_the_sign_up_form_backwards() {
    let mut lobby = Lobby::new();
    lobby.toggle_registering();
    assert_eq!(lobby.focus(), Field::Email);
    lobby.cycle_focus(Tab::Back);
    assert_eq!(
        lobby.focus(),
        Field::Password,
        "back from the first is last"
    );
    lobby.cycle_focus(Tab::Back);
    assert_eq!(lobby.focus(), Field::DisplayName);
    lobby.cycle_focus(Tab::Back);
    assert_eq!(lobby.focus(), Field::Email);
}

/// A ring of two reverses to itself, so the log-in form answers Tab and
/// ⇧Tab alike — which is what a browser does with two fields as well.
#[test]
fn shift_tab_on_the_log_in_form_is_tab() {
    let mut lobby = Lobby::new();
    lobby.cycle_focus(Tab::Back);
    assert_eq!(lobby.focus(), Field::Password);
    lobby.cycle_focus(Tab::Back);
    assert_eq!(lobby.focus(), Field::Email);
}

/// Tabbing into a field selects it, so the next character replaces what
/// is there. An append-only field could not do that, which is why an
/// address typed one letter wrong had to be deleted back to the mistake.
#[test]
fn tabbing_into_a_field_selects_what_is_in_it() {
    let mut lobby = Lobby::new();
    lobby.set_field(Field::Password, "wrong");
    lobby.cycle_focus(Tab::Next);
    assert_eq!(lobby.focus(), Field::Password);
    assert_eq!(lobby.buffer(Field::Password).selection(), Some(0..5));
    lobby.type_char('r');
    assert_eq!(lobby.field(Field::Password), "r");
}

/// Clicking into a field is not tabbing into it: the caret is placed,
/// nothing is selected, and typing goes on from there.
#[test]
fn clicking_into_a_field_does_not_select_it() {
    let mut lobby = Lobby::new();
    lobby.set_field(Field::Password, "half");
    lobby.focus_on(Field::Password);
    assert_eq!(lobby.buffer(Field::Password).selection(), None);
    lobby.type_char('!');
    assert_eq!(lobby.field(Field::Password), "half!");
}

/// What a browser's `<input>` reports after the player moved its caret:
/// the text is unchanged and only the caret moved, which
/// [`Lobby::set_field`] discards as "no change".
#[test]
fn the_platform_may_move_the_caret_without_changing_the_text() {
    let mut lobby = Lobby::new();
    lobby.set_field(Field::Email, "mail@example.com");
    lobby.set_field_at(Field::Email, "mail@example.com", 4, Some(0));
    assert_eq!(lobby.buffer(Field::Email).cursor(), 4);
    assert_eq!(lobby.buffer(Field::Email).selection(), Some(0..4));
    lobby.type_char('n');
    assert_eq!(lobby.field(Field::Email), "n@example.com");
}

/// The caret is a caret and not an append cursor: a correction made in
/// the middle of an address lands in the middle of it.
#[test]
fn typing_lands_at_the_caret_and_backspace_takes_what_is_before_it() {
    let mut lobby = Lobby::new();
    lobby.set_field_at(Field::Email, "mailexample.com", 4, None);
    lobby.type_char('@');
    assert_eq!(lobby.field(Field::Email), "mail@example.com");
    lobby.backspace();
    assert_eq!(lobby.field(Field::Email), "mailexample.com");
    lobby.delete_forward();
    assert_eq!(lobby.field(Field::Email), "mailxample.com");
    lobby.move_caret(Reach::Line, Dir::Left, false);
    lobby.type_char('e');
    assert_eq!(
        lobby.field(Field::Email),
        "emailxample.com",
        "Home, then type"
    );
}

#[test]
fn tab_skips_the_display_name_when_logging_in() {
    let mut lobby = Lobby::new();
    assert_eq!(lobby.focus(), Field::Email);
    lobby.cycle_focus(Tab::Next);
    assert_eq!(lobby.focus(), Field::Password);
    lobby.cycle_focus(Tab::Next);
    assert_eq!(lobby.focus(), Field::Email);
    lobby.toggle_registering();
    lobby.cycle_focus(Tab::Next);
    assert_eq!(lobby.focus(), Field::DisplayName);
}

#[test]
fn leaving_the_sign_up_form_moves_the_caret_off_a_hidden_field() {
    let mut lobby = Lobby::new();
    lobby.toggle_registering();
    lobby.focus_on(Field::DisplayName);
    lobby.toggle_registering();
    assert_eq!(lobby.focus(), Field::Password);
}

#[test]
fn a_gateway_that_takes_no_sign_ups_offers_none() {
    let mut lobby = Lobby::new();
    lobby.set_registration_enabled(false);
    lobby.toggle_registering();
    assert_eq!(lobby.screen(), &Screen::SignIn { registering: false });
    assert_eq!(lobby.status(), "this gateway is not taking new accounts");
}

#[test]
fn a_form_already_registering_survives_the_config_arriving_late() {
    let mut lobby = Lobby::new();
    lobby.toggle_registering();
    lobby.set_registration_enabled(false);
    assert_eq!(
        lobby.screen(),
        &Screen::SignIn { registering: false },
        "the offer is withdrawn, not left dangling"
    );
}

#[test]
fn typing_lands_in_the_focused_field_and_control_keys_do_not() {
    let mut lobby = Lobby::new();
    lobby.type_char('h');
    lobby.type_char('\n');
    lobby.type_char('i');
    assert_eq!(lobby.field(Field::Email), "hi");
    lobby.backspace();
    assert_eq!(lobby.field(Field::Email), "h");
    lobby.backspace();
    lobby.backspace();
    assert_eq!(lobby.field(Field::Email), "", "an empty field survives");
}

#[test]
fn a_selection_never_points_past_the_end_of_a_refreshed_list() {
    let mut lobby = seated_lobby();
    lobby.select_deck(0);
    lobby.refresh();
    lobby.apply(LobbyEvent::Decks(vec![]));
    assert_eq!(lobby.selected(), None);
}

#[test]
fn a_deck_that_does_not_exist_cannot_be_selected() {
    let mut lobby = seated_lobby();
    lobby.select_deck(9);
    assert_eq!(lobby.selected(), Some(0));
}

#[test]
fn saving_a_deck_re_reads_the_list() {
    let mut lobby = seated_lobby();
    assert_eq!(lobby.create_deck("Starter", vec![]), None, "no empty decks");
    let rows = vec!["40 Island".to_string(), "20 Forest".to_string()];
    assert_eq!(
        lobby.create_deck("Starter", rows.clone()),
        Some(LobbyRequest::SaveDeck {
            deck_id: None,
            name: "Starter".to_string(),
            cards: rows,
            sideboard: vec![],
            commanders: Vec::new(),
        })
    );
    assert_eq!(
        lobby.apply(LobbyEvent::DeckSaved {
            deck_id: Some("d9".to_string())
        }),
        Some(LobbyRequest::ListDecks)
    );
}

#[test]
fn signing_out_forgets_the_token_and_everything_it_bought() {
    let mut lobby = seated_lobby();
    lobby.sign_out();
    assert_eq!(lobby.token(), None);
    assert!(lobby.decks().is_empty());
    assert!(lobby.games().is_empty());
    assert_eq!(lobby.selected(), None);
    assert_eq!(lobby.screen(), &Screen::SignIn { registering: false });
    assert_eq!(lobby.refresh(), None, "no token, no requests");
}

#[test]
fn a_failure_is_shown_and_nothing_else() {
    let mut lobby = seated_lobby();
    lobby.host(GameMode::Ai);
    assert_eq!(
        lobby.apply(LobbyEvent::Failed("no such deck".to_string())),
        None
    );
    assert_eq!(lobby.status(), "no such deck");
    assert_eq!(*lobby.screen(), Screen::Table, "we stay where we were");
    assert!(!lobby.busy(), "and the lobby is usable again");
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
/// A page of `n` nameless tables, enough to page through.
fn a_page(n: usize) -> Vec<GameSummary> {
    (0..n)
        .map(|i| GameSummary {
            id: format!("g{i}"),
            state: "waiting".to_string(),
            ..GameSummary::default()
        })
        .collect()
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

/// Opening the builder asks for the pool once. Coming back must not ask
/// again: it is the same few hundred cards, and the round trip would be
/// paid on every visit.
#[test]
fn the_card_pool_is_fetched_once() {
    let mut lobby = seated_lobby();
    assert_eq!(lobby.build_deck(), Some(LobbyRequest::LoadPool));
    assert_eq!(lobby.screen(), &Screen::Build);
    lobby.apply(LobbyEvent::Pool {
        cards: vec![PoolCard {
            index: 1,
            english_name: "Forest".to_string(),
            name: "Forest".to_string(),
            kinds: vec!["Land".to_string()],
            type_line: "Basic Land — Forest".to_string(),
            basic_land: true,
            coverage: Coverage::Implemented,
            ..PoolCard::default()
        }],
        has_text: false,
    });
    assert!(lobby.builder().loaded());
    lobby.close_builder();
    assert_eq!(lobby.build_deck(), None, "the pool is already here");
    assert_eq!(lobby.screen(), &Screen::Build);
}

/// Editing a saved deck asks for its rows — `GET /decks` lists counts, not
/// contents, so the builder cannot fill itself from the list.
#[test]
fn editing_a_deck_asks_for_its_rows() {
    let mut lobby = seated_lobby();
    lobby.apply(LobbyEvent::Decks(vec![DeckSummary {
        id: "deck-1".to_string(),
        name: "Burn".to_string(),
        cards: 2,
        sideboard: 0,
        commanders: Vec::new(),
    }]));
    lobby.apply(LobbyEvent::Games(GameListing::default()));
    assert_eq!(
        lobby.edit_deck(0),
        Some(LobbyRequest::LoadDeck {
            deck_id: "deck-1".to_string()
        })
    );
    assert_eq!(lobby.edit_deck(9), None, "no such deck");
}

/// A deck that arrives before the pool is held by name and resolves when
/// the pool lands — the two answers race, and neither order may lose rows.
#[test]
fn a_deck_loaded_before_the_pool_still_resolves() {
    let mut lobby = seated_lobby();
    assert_eq!(
        lobby.apply(LobbyEvent::DeckLoaded {
            id: "deck-1".to_string(),
            name: "Trees".to_string(),
            cards: vec!["3 Forest".to_string()],
            sideboard: vec![],
            commanders: Vec::new(),
        }),
        Some(LobbyRequest::LoadPool),
        "the rows arrived first; the pool is still needed"
    );
    assert!(
        lobby.builder().missing().is_empty(),
        "nothing is missing yet — the pool has not had its say"
    );
    lobby.apply(LobbyEvent::Pool {
        cards: vec![PoolCard {
            index: 1,
            english_name: "Forest".to_string(),
            name: "Forest".to_string(),
            kinds: vec!["Land".to_string()],
            type_line: "Basic Land — Forest".to_string(),
            basic_land: true,
            ..PoolCard::default()
        }],
        has_text: false,
    });
    assert_eq!(lobby.builder().name(), "Trees");
    assert_eq!(
        lobby.builder().counts().main,
        3,
        "the held row became a real entry once the pool arrived"
    );
    assert!(lobby.builder().missing().is_empty());
}

/// Saving from the builder goes through the builder's own rules, so a deck
/// the gateway would refuse never leaves the client.
#[test]
fn the_builder_refuses_to_save_what_the_gateway_would_reject() {
    let mut lobby = seated_lobby();
    lobby.build_deck();
    lobby.apply(LobbyEvent::Pool {
        cards: vec![PoolCard {
            index: 1,
            english_name: "Forest".to_string(),
            name: "Forest".to_string(),
            kinds: vec!["Land".to_string()],
            type_line: "Basic Land — Forest".to_string(),
            basic_land: true,
            ..PoolCard::default()
        }],
        has_text: false,
    });
    assert_eq!(lobby.save_deck(), None, "nameless and empty");
    lobby.builder_mut().set_name("Trees");
    lobby.builder_mut().add(0, Zone::Main);
    assert_eq!(
        lobby.save_deck(),
        Some(LobbyRequest::SaveDeck {
            deck_id: None,
            name: "Trees".to_string(),
            cards: vec!["1 Forest".to_string()],
            sideboard: vec![],
            commanders: Vec::new(),
        })
    );
    assert_eq!(
        lobby.apply(LobbyEvent::DeckSaved {
            deck_id: Some("d9".to_string())
        }),
        Some(LobbyRequest::ListDecks)
    );
    assert!(!lobby.builder().dirty(), "saving settles the deck");
    assert_eq!(
        lobby.builder().editing(),
        Some("d9"),
        "and it is now the deck being edited"
    );
    // So a second save edits that deck rather than filing a copy of it.
    // (Through the list refresh the save kicked off, which is what frees
    // the lobby to send anything at all.)
    lobby.apply(LobbyEvent::Decks(vec![]));
    lobby.apply(LobbyEvent::Games(GameListing::default()));
    lobby.builder_mut().set_name("Trees II");
    assert_eq!(
        lobby.save_deck(),
        Some(LobbyRequest::SaveDeck {
            deck_id: Some("d9".to_string()),
            name: "Trees II".to_string(),
            cards: vec!["1 Forest".to_string()],
            sideboard: vec![],
            commanders: Vec::new(),
        })
    );
}

/// Deleting a deck re-reads the list, or the one that is gone stays on
/// screen until something else happens to refresh it.
#[test]
fn deleting_a_deck_re_reads_the_list() {
    let mut lobby = seated_lobby();
    lobby.apply(LobbyEvent::Decks(vec![DeckSummary {
        id: "deck-1".to_string(),
        name: "Burn".to_string(),
        cards: 2,
        sideboard: 0,
        commanders: Vec::new(),
    }]));
    lobby.apply(LobbyEvent::Games(GameListing::default()));
    assert_eq!(
        lobby.delete_deck(0),
        Some(LobbyRequest::DeleteDeck {
            deck_id: "deck-1".to_string()
        })
    );
    assert_eq!(
        lobby.apply(LobbyEvent::DeckDeleted),
        Some(LobbyRequest::ListDecks)
    );
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
