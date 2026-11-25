//! What the routes read out of a request before they ask the lobby or the
//! database anything.

use super::*;
use crate::lobby::{LobbyGame, LobbySeat, LobbyState};
use axum::http::{HeaderMap, HeaderValue, StatusCode};

const FRAMING: &str = central_api::advent::framing::CAPABILITY_HEADER;

fn headers(name: &'static str, values: &[&[u8]]) -> HeaderMap {
    let mut map = HeaderMap::new();
    for value in values {
        map.append(name, HeaderValue::from_bytes(value).unwrap());
    }
    map
}

#[test]
fn a_bearer_token_is_what_follows_the_one_scheme() {
    let token = |value: &[u8]| bearer_token(&headers("authorization", &[value])).map(str::to_owned);
    assert_eq!(token(b"Bearer abc").as_deref(), Some("abc"));
    assert_eq!(token(b"Bearer  abc").as_deref(), Some(" abc"));
    for other in [
        &b"abc"[..],
        b"Bearer",
        b"bearer abc",
        b"Basic YTpi",
        b"BearerX abc",
        b" Bearer abc",
        b"Bearer \xff",
    ] {
        assert_eq!(token(other), None, "{other:?}");
    }
    assert_eq!(bearer_token(&HeaderMap::new()), None);
}

#[test]
fn the_framing_capability_is_said_once_and_exactly() {
    let says = |values: &[&[u8]]| event_looks::accepts_framing(&headers(FRAMING, values));
    assert!(says(&[b"1"]));
    let others: [&[&[u8]]; 8] = [
        &[],
        &[b"0"],
        &[b""],
        &[b"1 "],
        &[b"11"],
        &[b"true"],
        &[b"1", b"1"],
        &[b"1", b"0"],
    ];
    for other in others {
        assert!(!says(other), "{other:?}");
    }
}

fn table(state: LobbyState, spectators: bool) -> LobbyGame {
    let mut game = LobbyGame::blank("routes-fixture".into(), 0);
    game.state = state;
    game.allow_spectators = spectators;
    let mut seat = LobbySeat::open(0);
    seat.account_id = Some("account".into());
    game.seats = vec![seat, LobbySeat::open(1)];
    game
}

#[test]
fn a_spectator_is_let_into_a_running_table_that_allows_one() {
    let refused = |state, spectators| {
        may_watch(&table(state, spectators))
            .err()
            .map(|(status, _)| status)
    };
    assert_eq!(refused(LobbyState::Playing, true), None);
    assert_eq!(
        refused(LobbyState::Waiting, true),
        Some(StatusCode::CONFLICT)
    );
    assert_eq!(refused(LobbyState::Over, true), Some(StatusCode::CONFLICT));
    // A table that allows none says the same whatever it is doing, so its
    // state is not told to who may not watch it.
    for state in [LobbyState::Waiting, LobbyState::Playing, LobbyState::Over] {
        assert_eq!(refused(state, false), Some(StatusCode::FORBIDDEN));
    }
}

#[test]
fn only_an_account_still_playing_a_casual_game_is_a_participant() {
    let playing = table(LobbyState::Playing, true);
    assert!(event_looks::participant(&playing, 0));
    assert!(!event_looks::participant(&playing, 1), "an empty chair");
    assert!(!event_looks::participant(&playing, 2), "no such chair");
    for state in [LobbyState::Waiting, LobbyState::Over] {
        assert!(!event_looks::participant(&table(state, true), 0));
    }
    let mut left = table(LobbyState::Playing, true);
    left.seats[0].participation_ended = true;
    assert!(!event_looks::participant(&left, 0));
    let mut challenge = table(LobbyState::Playing, true);
    challenge.challenge = Some([1; 16]);
    assert!(!event_looks::participant(&challenge, 0));
}

#[test]
fn a_list_of_card_ids_keeps_the_well_formed_ones_lowercased_and_bounded() {
    let id = "b2c6aa39-2d2a-459c-a555-fb48ba993373";
    assert_eq!(
        text_ids(&format!(" {} ,,x,{id}0,',{id}", id.to_uppercase())),
        [id, id]
    );
    assert_eq!(text_ids(""), [] as [&str; 0]);
    // The bound counts what is kept: junk in front of it costs no card its
    // text, and a longer list is cut.
    let many = format!("junk,{}", vec![id; MAX_TEXT_IDS + 1].join(","));
    assert_eq!(text_ids(&many).len(), MAX_TEXT_IDS);
}
