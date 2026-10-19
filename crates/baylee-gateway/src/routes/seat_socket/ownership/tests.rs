//! Deterministic interleavings across the asynchronous admission boundary.

use super::*;
use crate::{Envelope, lobby::LobbySeat};
use tokio::sync::mpsc::{UnboundedReceiver, error::TryRecvError};

fn game() -> (LobbyGame, UnboundedReceiver<Envelope>) {
    let (link, frames) = tokio::sync::mpsc::unbounded_channel();
    let mut game = LobbyGame::blank("ownership-fixture".into(), 0);
    let mut seat = LobbySeat::open(0);
    seat.seat_token_hash = Some("first-capability".into());
    game.seats.push(seat);
    game.engine = Some(link);
    (game, frames)
}

fn reserved(game: &mut LobbyGame) -> Ownership {
    let hash = game.seats[0].seat_token_hash.clone().unwrap();
    Ownership::reserve(game, 0, &hash).unwrap()
}

fn attached(frames: &mut UnboundedReceiver<Envelope>) {
    assert!(matches!(
        frames.try_recv().unwrap().msg,
        Some(v1::envelope::Msg::SeatAttached(v1::SeatAttached {
            seat: 0,
            resync: false
        }))
    ));
}

fn detached(frames: &mut UnboundedReceiver<Envelope>) {
    assert!(matches!(
        frames.try_recv().unwrap().msg,
        Some(v1::envelope::Msg::SeatDetached(v1::SeatDetached {
            seat: 0,
            left: false
        }))
    ));
}

fn empty(frames: &mut UnboundedReceiver<Envelope>) {
    assert!(matches!(frames.try_recv(), Err(TryRecvError::Empty)));
}

fn action() -> v1::envelope::Msg {
    v1::envelope::Msg::SeatFrame(v1::SeatFrame {
        seat: 0,
        envelope: bytes::Bytes::from_static(b"action"),
    })
}

#[test]
fn delayed_admission_cannot_take_back_a_newer_socket_even_with_the_same_token() {
    let (mut game, mut frames) = game();
    let slow = reserved(&mut game);
    let newer = reserved(&mut game);
    newer.attach(&mut game, 0).unwrap();
    attached(&mut frames);
    assert!(
        slow.attach(&mut game, 0).is_none(),
        "old Central/engine wait completed late"
    );
    assert!(newer.active(&game, 0));
    empty(&mut frames);
}

#[test]
fn replaced_socket_cannot_forward_resync_or_detach_the_current_socket() {
    let (mut game, mut frames) = game();
    let first = reserved(&mut game);
    let link = first.attach(&mut game, 0).unwrap();
    attached(&mut frames);
    let newer = reserved(&mut game);
    newer.attach(&mut game, 0).unwrap();
    attached(&mut frames);
    assert!(
        !first.active(&game, 0),
        "old socket also loses private broadcasts"
    );
    assert!(!first.send(&game, 0, &link, action()));
    assert!(!first.send(
        &game,
        0,
        &link,
        v1::envelope::Msg::SeatAttached(v1::SeatAttached {
            seat: 0,
            resync: true
        })
    ));
    first.detach(&mut game, 0, &link);
    empty(&mut frames);
    assert!(newer.send(&game, 0, &link, action()));
    assert!(matches!(
        frames.try_recv().unwrap().msg,
        Some(v1::envelope::Msg::SeatFrame(_))
    ));
    newer.detach(&mut game, 0, &link);
    detached(&mut frames);
    newer.detach(&mut game, 0, &link);
    empty(&mut frames);
}

#[test]
fn rotation_during_authorization_blocks_forwarding_but_a_real_departure_still_detaches() {
    let (mut game, mut frames) = game();
    let first = reserved(&mut game);
    let link = first.attach(&mut game, 0).unwrap();
    attached(&mut frames);
    game.seats[0].seat_token_hash = Some("new-capability".into());
    let newer = reserved(&mut game);
    assert!(
        !first.send(&game, 0, &link, action()),
        "an earlier async authorization is not enough"
    );
    first.detach(&mut game, 0, &link);
    newer.attach(&mut game, 0).unwrap();
    detached(&mut frames);
    attached(&mut frames);
    first.detach(&mut game, 0, &link);
    assert!(newer.active(&game, 0));
    empty(&mut frames);
}

#[test]
fn rotated_token_refuses_an_upgrade_that_already_passed_authorization() {
    let (mut game, mut frames) = game();
    let old = reserved(&mut game);
    game.seats[0].seat_token_hash = Some("replacement-capability".into());
    assert!(old.attach(&mut game, 0).is_none());
    assert!(Ownership::reserve(&mut game, 0, "first-capability").is_none());
    empty(&mut frames);
}

#[test]
fn failed_replacement_does_not_silence_the_previous_owners_departure() {
    let (mut game, mut frames) = game();
    let first = reserved(&mut game);
    let link = first.attach(&mut game, 0).unwrap();
    attached(&mut frames);
    let _unfinished = reserved(&mut game);
    assert!(first.active(&game, 0));
    first.detach(&mut game, 0, &link);
    detached(&mut frames);
    empty(&mut frames);
}

#[test]
fn resetting_a_chair_cannot_reuse_a_previous_socket_generation() {
    let (mut game, mut frames) = game();
    let first = reserved(&mut game);
    let link = first.attach(&mut game, 0).unwrap();
    attached(&mut frames);
    game.seats[0].vacate();
    game.seats[0].seat_token_hash = Some("first-capability".into());
    let newer = reserved(&mut game);
    newer.attach(&mut game, 0).unwrap();
    attached(&mut frames);
    first.detach(&mut game, 0, &link);
    assert!(!first.send(&game, 0, &link, action()));
    assert!(first.attach(&mut game, 0).is_none());
    assert!(newer.active(&game, 0));
    empty(&mut frames);
}

#[test]
fn terminal_participation_keeps_final_views_but_cannot_reopen_or_act() {
    let (mut game, mut frames) = game();
    let first = reserved(&mut game);
    let link = first.attach(&mut game, 0).unwrap();
    attached(&mut frames);
    game.seats[0].participation_ended = true;
    assert!(
        first.active(&game, 0),
        "the current socket can receive its final view"
    );
    assert!(Ownership::reserve(&mut game, 0, "first-capability").is_none());
    assert!(
        first.attach(&mut game, 0).is_none(),
        "a preauthorized delayed upgrade cannot revive a terminal seat"
    );
    assert!(!first.send(&game, 0, &link, action()));
    assert!(!first.send(
        &game,
        0,
        &link,
        v1::envelope::Msg::SeatAttached(v1::SeatAttached {
            seat: 0,
            resync: true
        })
    ));
    empty(&mut frames);
}
