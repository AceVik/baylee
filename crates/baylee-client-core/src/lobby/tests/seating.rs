//! What a `SeatHandover` is worth. A room is not started by being sat at — neither the one we opened nor the one we joined — so the screen turns `Seated` when the listing says the table is playing and not a page earlier, the exception being the house, which has nobody to wait for. The chair that outlived the client holding it is asked back for here as well, once however often the list is read, and so is the way out again: standing up, a rematch pressed on one side of the unseating and spent on the other, and a sign-out forgetting the table we were still waiting at. The requests that put us at a table are `rooms`, the account's own sign-out is `sign_in`, and how a refusal is *read* is `status`.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

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
