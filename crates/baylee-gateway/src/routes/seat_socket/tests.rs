//! Which chair a capability opens, and what a seat may send.

use super::*;
use crate::lobby::LobbySeat;
use std::time::{Duration, Instant};

fn game(tokens: &[Option<&str>]) -> LobbyGame {
    let mut game = LobbyGame::blank("seat-fixture".into(), 0);
    game.seats = tokens
        .iter()
        .enumerate()
        .map(|(seat, token)| {
            let mut chair = LobbySeat::open(seat);
            chair.seat_token_hash = token.map(auth::token_hash);
            chair
        })
        .collect();
    game
}

#[test]
fn a_token_opens_its_own_chair_and_nothing_else_does() {
    let game = game(&[None, Some("first"), Some("second")]);
    assert_eq!(seat_of_token(&game, "first"), Some(1));
    assert_eq!(seat_of_token(&game, "second"), Some(2));
    for wrong in ["", "firs", "first ", "FIRST", "third"] {
        assert_eq!(seat_of_token(&game, wrong), None, "{wrong:?}");
    }
    // The hash the gateway keeps is no token: a leaked one opens nothing.
    assert_eq!(seat_of_token(&game, &auth::token_hash("first")), None);
}

#[test]
fn a_chair_without_a_token_is_opened_by_none() {
    let mut game = game(&[None]);
    assert_eq!(seat_of_token(&game, ""), None);
    assert!(!holds_token(&game, 0));
    assert!(!holds_token(&game, 7), "no such chair");
    game.seats[0].seat_token_hash = Some(auth::token_hash("x"));
    assert!(holds_token(&game, 0));
}

struct Budget {
    meter: seatrate::Meter,
    frames: seatrate::Allowance,
    volume: seatrate::Allowance,
}

/// A budget that gives nothing back while the test runs.
fn budget(frames: u32, kib: u32) -> Budget {
    let now = Instant::now();
    let day = Duration::from_hours(24);
    Budget {
        meter: seatrate::Meter::new(now),
        frames: seatrate::Allowance::every(now, day, frames),
        volume: seatrate::Allowance::every(now, day, kib),
    }
}

impl Budget {
    fn admits(&mut self, message: &Message) -> bool {
        within_allowance(&mut self.meter, &mut self.frames, &mut self.volume, message)
    }
}

fn binary(len: usize) -> Message {
    Message::Binary(vec![0; len].into())
}

#[test]
fn a_seat_spends_one_frame_for_each_message_of_any_kind() {
    let mut budget = budget(3, 0);
    assert!(budget.admits(&binary(1023)));
    assert!(budget.admits(&Message::Text("x".into())));
    assert!(budget.admits(&Message::Ping(Vec::new().into())));
    assert!(!budget.admits(&binary(0)), "an empty frame is a frame");
}

#[test]
fn a_seat_spends_its_volume_in_whole_kibibytes() {
    let mut budget = budget(100, 5);
    assert!(budget.admits(&binary(1023)), "under one costs a frame only");
    assert!(budget.admits(&binary(2 * 1024)));
    assert!(budget.admits(&binary(3 * 1024 + 1023)));
    assert!(!budget.admits(&binary(1024)), "the volume is spent");
    assert!(budget.admits(&binary(1023)), "and a small frame is not");
}

#[test]
fn one_frame_larger_than_the_whole_volume_is_refused() {
    let mut budget = budget(100, 4);
    assert!(!budget.admits(&binary(5 * 1024)));
}
