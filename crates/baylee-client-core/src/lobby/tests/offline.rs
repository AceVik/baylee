//! Play with no account behind it. Every intent method used to read the token as its proof that a request could go anywhere at all, so what is asserted here is that the offline performer still reaches the builder, the room and the start button, and that a table which is its own process waits for the person at the keyboard rather than for an opponent who cannot come and asks no gateway for a chair back. The gateway's answer to those same two events is kept beside them as the counter-half, because the sentence only means something against the one it is not. Anything that holds whether or not there is an account behind the lobby belongs to `rooms` and `seating`; only a claim that turns on there being nobody to ask lives here.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

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

/// #270: a sign-out forgets the pool, offline play's too. Offline has no
/// session to lose and a pool of its own, so coming back asks this process
/// for it again, and the builder opens as it did.
#[test]
fn offline_play_asks_for_its_pool_again_after_a_sign_out() {
    let mut lobby = offline_lobby();
    assert_eq!(lobby.build_deck(), Some(LobbyRequest::LoadPool));
    lobby.apply(LobbyEvent::Pool {
        cards: vec![PoolCard {
            index: 1,
            english_name: "Forest".to_string(),
            name: "Forest".to_string(),
            ..PoolCard::default()
        }],
        has_text: false,
    });
    assert!(lobby.builder().loaded());
    lobby.sign_out();
    assert_eq!(lobby.play_offline(), Some(LobbyRequest::ListDecks));
    assert_eq!(lobby.token(), None, "still nobody's session");
    assert_eq!(lobby.build_deck(), Some(LobbyRequest::LoadPool));
    assert_eq!(*lobby.screen(), Screen::Build);
}

/// Signing out of offline play puts the sign-in screen back in charge.
#[test]
fn leaving_offline_play_takes_the_performer_with_it() {
    let mut lobby = offline_lobby();
    lobby.sign_out();
    assert_eq!(lobby.refresh(), None, "there is nobody to ask again");
}
