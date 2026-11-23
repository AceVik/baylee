//! What one press in the lobby says to the gateway: exactly the one thing it is about, once at a time, and only from somebody who is signed in. And the two ways out of a seat and of an account.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

use baylee_core::preset::{RoomSetup, RoomUpdate};

fn update() -> RoomUpdate {
    RoomUpdate {
        name: "Friday".into(),
        chairs: 4,
        password: None,
        setup: RoomSetup::default(),
    }
}

/// One press.
type Say = fn(&mut Lobby) -> Option<LobbyRequest>;

/// Every statement about a room, by what it sends.
fn statements() -> Vec<(&'static str, Say)> {
    vec![
        ("configure", |l| l.configure_room("g", update())),
        ("list", |l| l.choose_room_list("g", None)),
        ("ready", |l| l.set_ready("g", true)),
        ("start", |l| l.start_room("g")),
        ("hand over", |l| l.hand_over("g", 1)),
        ("seat", |l| l.set_seat("g", 1, Some(SeatKind::Ai), None)),
        ("team", |l| l.seat_team("g", 1, 2)),
        ("deck", |l| l.seat_deck("g", 1)),
        ("deck at", |l| l.seat_deck_at("g", 1, 0, false)),
        ("own deck at", |l| l.seat_deck_at("g", 1, 0, true)),
        ("leave", |l| l.leave_table("g")),
        ("rematch", |l| l.rematch("g")),
        ("return", |l| l.return_to_game("g")),
        ("join", |l| l.join("g")),
        ("host", |l| l.host(GameMode::Open)),
    ]
}

#[test]
fn every_statement_about_a_room_waits_for_the_one_before_it() {
    for (name, say) in statements() {
        let mut lobby = seated_lobby();
        assert!(say(&mut lobby).is_some(), "{name}");
        assert!(lobby.busy(), "{name}");
        for (next, say) in statements() {
            assert_eq!(say(&mut lobby), None, "{next} while {name} is under way");
        }
        assert!(!lobby.watch_table("g"), "nor is a table watched meanwhile");
    }
}

#[test]
fn nobody_signed_in_says_nothing_to_a_room() {
    for (name, say) in statements() {
        let mut lobby = Lobby::new();
        assert_eq!(say(&mut lobby), None, "{name}");
        assert!(!lobby.busy(), "{name}");
    }
    assert!(!Lobby::new().watch_table("g"));
}

/// A `SetSeat` that changes nothing, for a chair of the room `g`.
fn seat(number: u32) -> LobbyRequest {
    LobbyRequest::SetSeat {
        game_id: "g".into(),
        seat: number,
        kind: None,
        ai: None,
        deck_id: None,
        team: None,
    }
}

#[test]
fn a_statement_about_a_chair_changes_only_what_it_names() {
    let ask = |say: fn(&mut Lobby) -> Option<LobbyRequest>| say(&mut seated_lobby()).unwrap();
    let changed = |change: fn(&mut LobbyRequest)| {
        let mut request = seat(2);
        change(&mut request);
        request
    };
    assert_eq!(ask(|l| l.set_seat("g", 2, None, None)), seat(2));
    assert_eq!(
        ask(|l| l.set_seat("g", 2, Some(SeatKind::Ai), Some("expert".into()))),
        changed(|r| if let LobbyRequest::SetSeat { kind, ai, .. } = r {
            *kind = Some(SeatKind::Ai);
            *ai = Some("expert".into());
        })
    );
    assert_eq!(
        ask(|l| l.seat_team("g", 2, 1)),
        changed(|r| if let LobbyRequest::SetSeat { team, .. } = r {
            *team = Some(1);
        })
    );
    assert_eq!(
        ask(|l| l.seat_deck("g", 2)),
        changed(|r| if let LobbyRequest::SetSeat { deck_id, .. } = r {
            *deck_id = Some("d1".into());
        })
    );
}

#[test]
fn a_chair_is_given_no_deck_that_is_not_there() {
    let mut lobby = seated_lobby();
    assert_eq!(lobby.seat_deck_at("g", 1, 9, false), None);
    assert!(!lobby.busy(), "a refused intent leaves nothing in flight");
    lobby.apply(LobbyEvent::Decks(Vec::new()));
    lobby.apply(LobbyEvent::Games(GameListing::default()));
    assert_eq!(lobby.seat_deck("g", 1), None);
    assert_eq!(lobby.tone(), Tone::Refusal);
    assert!(!lobby.busy());
}

#[test]
fn a_rooms_settings_and_its_list_go_to_the_room_they_are_about() {
    assert_eq!(
        seated_lobby().configure_room("g", update()),
        Some(LobbyRequest::ConfigureRoom {
            game_id: "g".into(),
            update: update(),
        })
    );
    for list in [None, Some("list-1".to_owned())] {
        assert_eq!(
            seated_lobby().choose_room_list("g", list.clone()),
            Some(LobbyRequest::ChooseRoomList {
                game_id: "g".into(),
                list_id: list,
            })
        );
    }
    assert_eq!(
        seated_lobby().leave_table("g"),
        Some(LobbyRequest::LeaveGame {
            game_id: "g".into()
        })
    );
}

#[test]
fn a_game_is_returned_to_from_the_lobby_of_a_gateway_only() {
    assert_eq!(
        seated_lobby().return_to_game("g"),
        Some(LobbyRequest::TakeSeat {
            game_id: "g".into()
        })
    );
    assert_eq!(offline_lobby().return_to_game("g"), None);
    assert!(!offline_lobby().watch_table("g"));
    // Already in a seat: another is not taken beside it.
    let mut watching = seated_lobby();
    assert!(watching.watch_table("g"));
    assert!(!watching.watch_table("h"));
    assert_eq!(watching.return_to_game("g"), None);
}

#[test]
fn a_wish_to_play_again_is_spent_on_one_request() {
    let mut lobby = seated_lobby();
    assert_eq!(lobby.take_rematch(), None);
    lobby.want_rematch("g");
    assert_eq!(lobby.rematch_wanted(), Some("g"));
    assert_eq!(
        lobby.take_rematch(),
        Some(LobbyRequest::Rematch {
            game_id: "g".into()
        })
    );
    assert_eq!(lobby.rematch_wanted(), None);
}

/// One way out of a seat.
type Leave = fn(&mut Lobby);

#[test]
fn the_ways_out_of_a_seat_differ_in_tone_alone() {
    let out: [(Leave, Tone); 2] = [
        (|l| l.unseat("the table is gone"), Tone::Refusal),
        (|l| l.stand_up(Phrase::SignedOut, &[]), Tone::Note),
    ];
    for (leave, tone) in out {
        let mut lobby = seated_lobby();
        assert!(lobby.watch_table("g"));
        leave(&mut lobby);
        assert!(matches!(lobby.screen(), Screen::Table));
        assert_eq!(lobby.tone(), tone);
        assert!(!lobby.busy());
        assert!(!lobby.status().is_empty());
    }
}

#[test]
fn a_hint_that_the_seat_is_lost_never_signs_anybody_in() {
    let mut lobby = Lobby::new();
    lobby.unseat("the table is gone");
    assert!(matches!(lobby.screen(), Screen::SignIn { .. }));
}

#[test]
fn what_the_gateway_says_of_the_player_is_new_only_once() {
    let mut lobby = seated_lobby();
    let me = strips::Me {
        handle: "alice#0001".into(),
        guest: false,
    };
    assert!(lobby.set_me(me.clone()));
    assert!(!lobby.set_me(me.clone()));
    assert_eq!(lobby.me(), Some(&me));
    let stats = strips::LobbyStats {
        players_online: 3,
        ..strips::LobbyStats::default()
    };
    assert!(lobby.set_stats(stats));
    assert!(!lobby.set_stats(stats));
    assert_eq!(lobby.stats(), Some(&stats));
    lobby.sign_out();
    assert_eq!(lobby.me(), None, "the next account is somebody else");
    assert_eq!(lobby.stats(), None);
}

#[test]
fn an_account_central_holds_is_deleted_without_a_password_of_this_gateway() {
    let mut lobby = seated_lobby();
    assert!(lobby.deletion_needs_password());
    lobby.set_central_bound(true);
    assert!(lobby.central_bound());
    assert!(!lobby.deletion_needs_password());
    lobby.ask_to_delete_account();
    assert_eq!(
        lobby.delete_account(),
        Some(LobbyRequest::DeleteAccount { password: None })
    );
    assert_eq!(lobby.delete_account(), None, "and only once");
}

#[test]
fn nobody_without_an_account_is_asked_to_delete_one() {
    for mut lobby in [Lobby::new(), offline_lobby()] {
        lobby.ask_to_delete_account();
        assert!(lobby.deleting_account().is_none());
        assert_eq!(lobby.delete_account(), None);
        assert!(!lobby.deletion_needs_password());
        assert_eq!(lobby.depart(), None);
        assert_eq!(lobby.sign_out(), None);
    }
}

#[test]
fn departing_names_the_session_and_keeps_it_while_signing_out_ends_it() {
    let mut lobby = seated_lobby();
    assert_eq!(
        lobby.depart(),
        Some(LobbyRequest::Depart {
            token: "tok".into()
        })
    );
    assert_eq!(lobby.token(), Some("tok"));
    assert_eq!(
        lobby.sign_out(),
        Some(LobbyRequest::LogOut {
            token: "tok".into()
        })
    );
    assert_eq!(lobby.token(), None);
    assert_eq!(lobby.depart(), None);
}
