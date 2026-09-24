//! The two ways out of a finished game, and the road back to the lobby. Which of them is offered depends on whether there is a table to ask another of, they are hung in the row `hud::spawn_finish` left for them rather than in a bar the lobby spawns over the felt, and a keyboard reaches both — `Enter` and `Space` answer the lead way, `Esc` the quiet one, and the rematch a press records is what tells them apart. Coming back is the other half: the lobby lands where it was, the table that was played is gone from the list, and nothing is refused on the way. `DuelPlugin` is not in these apps, so the phase is moved by hand and only the row's own buttons are read; what the duel draws above it is not tested here.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

/// The end screen has a way out for somebody with no pointer.
///
/// `DuelSet::Input` stops at `DuelPhase::Playing`, so on the verdict sheet
/// every key did nothing at all: it was the one screen in the client a
/// keyboard could reach and not leave. Offline there is a single button, so
/// all three of Escape, Space and Enter are the same door — and each is
/// pressed on its own frame, because one that fired on the wrong action would
/// otherwise hide behind another that fired correctly.
#[test]
fn a_keyboard_can_leave_the_end_screen() {
    for key in [KeyCode::Escape, KeyCode::Space, KeyCode::Enter] {
        let mut app = headless();
        app.world_mut().resource_mut::<LobbyState>().offline =
            Some(super::offline::Offline::without_a_file());
        to_gateway_face(&mut app);
        tap_control(&mut app, "play offline", |p| *p == Press::PlayOffline);
        tap_control(&mut app, "play the house", |p| {
            *p == Press::Host(GameMode::Ai)
        });
        for phase in [DuelPhase::Playing, DuelPhase::Finished] {
            app.world_mut()
                .resource_mut::<NextState<DuelPhase>>()
                .set(phase);
            app.update();
        }
        // The sheet is the duel's; this app has no `DuelPlugin`, so only the
        // row's buttons are here — which is all this reads.
        let mut exits = app.world_mut().query::<&Press>();
        assert!(
            exits.iter(app.world()).any(|p| *p == Press::Leave),
            "the way back is drawn"
        );
        app.world_mut()
            .resource_mut::<Messages<DuelCommand>>()
            .clear();

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.update();
        let closed: Vec<DuelCommand> = app
            .world_mut()
            .resource_mut::<Messages<DuelCommand>>()
            .drain()
            .collect();
        assert!(
            closed.iter().any(|c| matches!(c, DuelCommand::Close)),
            "{key:?} left the end screen: {closed:?}"
        );
    }
}

/// The branch the test above cannot reach: which way out a key *chooses*.
///
/// Offline there is only ever one button on the sheet, so all three keys land
/// on it and the selection in `leave_keys` never runs. At a gateway's table
/// there are two, and the rule is the sheet's own: `Enter` and `Space` answer
/// the **lead** way — *play again* — while `Esc` is the quiet one and goes
/// back to the lobby. Both write a `Close`; what separates them is whether a
/// rematch was recorded for the way out to send.
#[test]
fn a_keyboard_answers_the_lead_way_out_and_escape_still_leaves() {
    for (key, rematch) in [
        (KeyCode::Enter, Some("g1")),
        (KeyCode::Space, Some("g1")),
        (KeyCode::Escape, None),
    ] {
        let mut app = headless();
        stocked(&mut app);
        {
            let mut state = app.world_mut().resource_mut::<LobbyState>();
            state.lobby.host(GameMode::Ai);
            state.lobby.apply(LobbyEvent::Seated(SeatHandover {
                game_id: "g1".to_string(),
                seat: 0,
                seat_token: "st".to_string(),
                local: false,
            }));
            // Stand in for the dial that opened the game now ending, so the
            // poller leaves the socket alone.
            state.connected = true;
        }
        phase(&mut app, DuelPhase::Finished);
        let found = presses(&mut app);
        assert!(
            found.contains(&Press::PlayAgain) && found.contains(&Press::Leave),
            "both ways out are drawn: {found:?}"
        );
        app.world_mut()
            .resource_mut::<Messages<DuelCommand>>()
            .clear();

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.update();
        let closed: Vec<DuelCommand> = app
            .world_mut()
            .resource_mut::<Messages<DuelCommand>>()
            .drain()
            .collect();
        assert!(
            closed.iter().any(|c| matches!(c, DuelCommand::Close)),
            "{key:?} left the end screen: {closed:?}"
        );
        assert_eq!(
            app.world().resource::<LobbyState>().lobby.rematch_wanted(),
            rematch,
            "{key:?} took the wrong way out"
        );
    }
}

/// And coming back from it leaves the lobby as it found it.
///
/// The finished offline table used to stay in the list as `yours` and
/// `"playing"`, so the very next listing made `Lobby::reclaim_a_seat` ask for
/// the ticket to that chair — a request offline can only refuse, in English,
/// in red, in the corner of a lobby of a player who had done nothing but
/// finish a game. Both halves are one cause and this is the wiring for it:
/// `came_back` is where the host stops existing, so it is where the table
/// does too.
#[test]
fn coming_back_from_an_offline_duel_leaves_no_table_and_no_refusal() {
    let mut app = headless();
    app.world_mut().resource_mut::<LobbyState>().offline =
        Some(super::offline::Offline::without_a_file());
    to_gateway_face(&mut app);
    tap_control(&mut app, "play offline", |p| *p == Press::PlayOffline);
    tap_control(&mut app, "play the house", |p| {
        *p == Press::Host(GameMode::Ai)
    });
    // One tap seats you without re-reading the list, so the room is not in
    // `games()` yet — it is the refresh on the way *back* that lists it, and
    // that is the listing this is about.

    // `DuelPlugin` is not in this app, so the phase is moved by hand — what
    // `DuelCommand::Open` and the leave button eventually become. `came_back`
    // hangs on `OnEnter(Closed)`.
    for phase in [DuelPhase::Playing, DuelPhase::Closed] {
        app.world_mut()
            .resource_mut::<NextState<DuelPhase>>()
            .set(phase);
        app.update();
    }
    // Decks, then tables: the refresh goes round the mailbox once per frame,
    // exactly as a gateway's answers would.
    app.update();
    app.update();

    let state = app.world().resource::<LobbyState>();
    assert_eq!(*state.lobby.screen(), Screen::Table, "back at the list");
    assert!(
        state.lobby.games().is_empty(),
        "the table it played at is gone: {:?}",
        state.lobby.games()
    );
    assert!(
        !state.lobby.decks().is_empty(),
        "and the refresh it came back through still ran"
    );
    assert_ne!(
        state.lobby.tone(),
        Tone::Refusal,
        "nothing was refused: {:?}",
        state.lobby.status()
    );
}

/// What the lobby says about the game it just came back from (#155).
///
/// The wording is `Lobby::stand_up_after`'s and is tested in client-core.
/// What is tested here is the **join** — that `came_back` reaches for it at
/// all — because a method nobody calls is this client's own recurring
/// defect, and the sibling test below is the reason it could go unnoticed:
/// with no `Duel` in the app, every path through `came_back` looks the same.
///
/// So both halves are asserted, and the second is not a formality. If
/// `came_back` took a plain `Res<Duel>` instead of an `Option`, Bevy would
/// skip the system outright in an app that has none — no failure, one
/// warning — and every other test in this file would keep passing while
/// `came_back` did nothing at all.
#[test]
fn the_lobby_says_which_way_the_game_went_and_not_only_that_it_is_over() {
    let result = baylee_engine::win::GameResult {
        winner: Some(baylee_engine::win::Victor::Player(PlayerId::new(1))),
        reason: baylee_engine::win::EndReason::LastPlayerStanding,
    };

    // With the duel the shell would be holding: the two fields `came_back`
    // reads, and nothing else. `DuelPlugin` is not in this app, so the
    // resource is inserted by hand — seat 1 outlived seat 0, so seat 0 lost.
    let mut app = came_back_from_a_duel(Some(crate::Duel {
        statics: Some(baylee_view::GameStatic {
            decision_secs: None,
            reconnect_secs: None,
            view_version: baylee_view::VIEW_VERSION,
            game_id: "g".into(),
            your_seat: PlayerId::new(0),
            seats: vec![baylee_view::SeatIdentity {
                player: PlayerId::new(0),
                display_name: "me".into(),
                is_ai: false,
                away: false,
                team: None,
            }],
            prints: vec![],
        }),
        interaction: Some(baylee_client_core::Interaction::new(
            baylee_engine::choice::Pending::GameOver(result),
            PlayerId::new(0),
        )),
        ..crate::Duel::default()
    }));
    assert_eq!(
        app.world().resource::<LobbyState>().lobby.status(),
        "You lost. Only one player left in the game",
    );

    // And with no duel to ask, which is an embedder with its own, and every
    // other test in this file. The old sentence is the fallback and not a
    // bug — what would be a bug is silence, or `came_back` not running.
    app = came_back_from_a_duel(None);
    let state = app.world().resource::<LobbyState>();
    assert_eq!(state.lobby.status(), "the game ended");
    assert_eq!(
        *state.lobby.screen(),
        Screen::Table,
        "and it still stood up, which is the half a skipped system would lose",
    );
}

/// Seats at an offline table, optionally installs a `Duel`, and comes back.
///
/// The phase is moved by hand for the reason the rest of this file gives:
/// `DuelPlugin` is not here, so nothing else writes `DuelPhase`.
fn came_back_from_a_duel(duel: Option<crate::Duel>) -> App {
    let mut app = headless();
    app.world_mut().resource_mut::<LobbyState>().offline =
        Some(super::offline::Offline::without_a_file());
    to_gateway_face(&mut app);
    tap_control(&mut app, "play offline", |p| *p == Press::PlayOffline);
    tap_control(&mut app, "play the house", |p| {
        *p == Press::Host(GameMode::Ai)
    });
    if let Some(duel) = duel {
        app.world_mut().insert_resource(duel);
    }
    for phase in [DuelPhase::Playing, DuelPhase::Closed] {
        app.world_mut()
            .resource_mut::<NextState<DuelPhase>>()
            .set(phase);
        app.update();
    }
    app
}

/// A duel against the house offers one way out, and it is the lead answer.
///
/// Found live rather than reasoned about: a conceded offline duel drew a
/// brass *play again* in the middle of its end screen, and pressing it would
/// have asked the gateway for another of a table the gateway has never heard
/// of. An offline seat is `Screen::Seated` like any other — `systems::poll`
/// branches on `handover.local`, not on the variant — so the test that
/// matters is the flag and not the screen.
#[test]
fn a_duel_against_the_house_offers_one_way_out_and_no_rematch() {
    let mut app = headless();
    stocked(&mut app);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.host(GameMode::Ai);
        state.lobby.apply(LobbyEvent::Seated(SeatHandover {
            game_id: "offline".to_string(),
            seat: 0,
            seat_token: String::new(),
            local: true,
        }));
        state.connected = true;
    }
    app.world_mut().spawn(crate::hud::FinishExits);
    phase(&mut app, DuelPhase::Finished);

    let found = presses(&mut app);
    assert!(
        !found.contains(&Press::PlayAgain),
        "there is no table to ask for another of: {found:?}"
    );
    assert!(found.contains(&Press::Leave), "{found:?}");
}

/// The ways out belong *in* the duel's end screen, not floating over the board.
///
/// `hud::spawn_finish` leaves one row marked `FinishExits` and this is the
/// only thing that joins the two plugins — no call, no ordering, no shared
/// resource. So the marker is put down by hand here and the buttons have to
/// find it: a lobby that went back to spawning a bar of its own over the
/// table would pass every other test in this file, and the screen would have
/// a hole in the middle of it with two buttons hovering above the felt.
#[test]
fn the_ways_out_are_put_in_the_row_the_end_screen_left_for_them() {
    let mut app = headless();
    stocked(&mut app);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.host(GameMode::Ai);
        state.lobby.apply(LobbyEvent::Seated(SeatHandover {
            game_id: "g1".to_string(),
            seat: 0,
            seat_token: "st".to_string(),
            local: false,
        }));
        state.connected = true;
    }
    let row = app.world_mut().spawn(crate::hud::FinishExits).id();
    phase(&mut app, DuelPhase::Finished);

    let mut buttons = app.world_mut().query::<(Entity, &Press)>();
    let ways: Vec<Entity> = buttons
        .iter(app.world())
        .filter(|(_, p)| matches!(p, Press::PlayAgain | Press::Leave))
        .map(|(e, _)| e)
        .collect();
    assert_eq!(ways.len(), 2, "both ways out");
    for way in ways {
        let parent = app
            .world()
            .entity(way)
            .get::<ChildOf>()
            .map(ChildOf::parent);
        assert_eq!(parent, Some(row), "a way out was hung somewhere else");
    }
    let mut bars = app
        .world_mut()
        .query_filtered::<Entity, With<LeaveButton>>();
    assert_eq!(
        bars.iter(app.world()).count(),
        0,
        "the fallback bar was built even though the screen had left a row"
    );

    // And it settles: the same frame twice must not hang a second pair.
    app.update();
    let mut again = app.world_mut().query::<&Press>();
    assert_eq!(
        again
            .iter(app.world())
            .filter(|p| matches!(p, Press::PlayAgain | Press::Leave))
            .count(),
        2,
        "the ways out were spawned again on the next frame"
    );
}

/// The whole *play again* path from the player's side: the button stands over
/// the finished game, pressing it and coming back asks the gateway for the
/// next table, and the ticket that answers seats them again.
///
/// Written as a press rather than as a hand-built request because these two
/// halves are exactly the pair that can be wired to nothing: a button no
/// system reads and a reader no button reaches both pass a test that calls
/// the method itself.
#[test]
fn playing_again_is_asked_for_from_the_button_over_a_finished_game() {
    let mut app = headless();
    stocked(&mut app);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        // A table against the house, for the reason the test above gives.
        state.lobby.host(GameMode::Ai);
        state.lobby.apply(LobbyEvent::Seated(SeatHandover {
            game_id: "g1".to_string(),
            seat: 0,
            seat_token: "st".to_string(),
            local: false,
        }));
        // Stand in for the dial that opened the game now ending.
        state.connected = true;
    }
    phase(&mut app, DuelPhase::Finished);
    let found = presses(&mut app);
    assert!(found.contains(&Press::PlayAgain), "{found:?}");
    assert!(found.contains(&Press::Leave), "leaving stays on offer");

    press(&mut app, Press::PlayAgain);
    assert_eq!(
        app.world().resource::<LobbyState>().lobby.rematch_wanted(),
        Some("g1"),
        "the button recorded nothing for the way out to send"
    );
    phase(&mut app, DuelPhase::Closed);
    assert_eq!(
        app.world().resource::<LobbyState>().lobby.rematch_wanted(),
        None,
        "and leaving the table spent it, rather than re-reading the list"
    );

    // The reply is an ordinary ticket, to a table that is not the one played.
    // A rematch room is a *room*: `POST …/rematch` marks the presser ready
    // and tries to start, so it is running only once the other player has
    // pressed too. Whoever presses first holds a good ticket to a table that
    // has not begun, which is why the ticket alone does not open a duel —
    // doing that is how a player came to be sitting in front of an empty
    // table with no way back to the lobby.
    app.world_mut().resource_mut::<LobbyState>().connected = true;
    app.world()
        .resource::<Mailbox>()
        .0
        .lock()
        .expect("mailbox")
        .push(Reply::Event(LobbyEvent::Seated(SeatHandover {
            game_id: "g2".to_string(),
            seat: 1,
            seat_token: "st2".to_string(),
            local: false,
        })));
    app.update();
    assert!(
        matches!(
            app.world().resource::<LobbyState>().lobby.screen(),
            Screen::Table
        ),
        "the seat is held while the room is still waiting"
    );

    // And the listing is what says the other player has pressed theirs.
    app.world()
        .resource::<Mailbox>()
        .0
        .lock()
        .expect("mailbox")
        .push(Reply::Event(LobbyEvent::Games(GameListing::of(vec![
            GameSummary {
                id: "g2".to_string(),
                state: "playing".to_string(),
                rematch: true,
                seats: vec![GameSeat {
                    seat: 1,
                    taken: true,
                    you: true,
                    ready: true,
                    ..GameSeat::default()
                }],
                ..GameSummary::default()
            },
        ]))));
    app.update();
    let Screen::Seated(handover) = app.world().resource::<LobbyState>().lobby.screen() else {
        panic!("the ticket seats the player again");
    };
    assert_eq!(handover.game_id, "g2");
}
