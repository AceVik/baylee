//! CR 400.7: an object that moves from one zone to another becomes a new
//! object with no memory of its previous existence. The id stays (this
//! engine keeps one id per card and bumps its version), so what has to be
//! checked is that everything the old object carried is gone: damage,
//! counters, tapped status, attachment, and that the version moved.

use super::{RegistryLookup, make_preset};
use crate::event::Cause;
use crate::object::{CounterKind, Status};
use crate::state::GameState;
use crate::zone::{Zone, ZoneLocation, ZonePosition};
use baylee_core::ids::{ObjectId, PlayerId};

const P0: PlayerId = PlayerId::new(0);

fn state(seed: u64) -> GameState {
    GameState::from_preset(&make_preset(seed), &RegistryLookup).expect("game starts")
}

fn go(state: &mut GameState, id: ObjectId, to: ZoneLocation) -> ObjectId {
    state
        .move_object(id, to, ZonePosition::Top, Cause::Effect)
        .expect("the move is legal")
}

/// A card from the top of seat 0's library, put onto the battlefield.
fn permanent(state: &mut GameState) -> ObjectId {
    let card = state.draw_cards(P0, 1)[0];
    go(state, card, ZoneLocation::Battlefield)
}

#[test]
fn a_move_bumps_the_version_each_time() {
    let mut state = state(201);
    let id = permanent(&mut state);
    let v0 = state.object(id).unwrap().version;
    let id = go(&mut state, id, ZoneLocation::Graveyard(P0));
    let v1 = state.object(id).unwrap().version;
    assert!(v1 > v0);
    let id = go(&mut state, id, ZoneLocation::Exile(P0));
    assert!(state.object(id).unwrap().version > v1);
}

#[test]
fn damage_does_not_follow_a_permanent_out_and_back() {
    let mut state = state(202);
    let id = permanent(&mut state);
    state.object_mut(id).unwrap().damage = 3;
    let id = go(&mut state, id, ZoneLocation::Hand(P0));
    let id = go(&mut state, id, ZoneLocation::Battlefield);
    assert_eq!(state.object(id).unwrap().damage, 0);
}

#[test]
fn counters_do_not_follow_a_permanent_into_the_graveyard() {
    let mut state = state(203);
    let id = permanent(&mut state);
    state
        .object_mut(id)
        .unwrap()
        .counters
        .set(CounterKind::P1P1, 2);
    let id = go(&mut state, id, ZoneLocation::Graveyard(P0));
    assert_eq!(state.object(id).unwrap().counters.get(CounterKind::P1P1), 0);
}

#[test]
fn a_tapped_permanent_comes_back_untapped() {
    let mut state = state(204);
    let id = permanent(&mut state);
    state.set_tapped(id, true);
    let id = go(&mut state, id, ZoneLocation::Exile(P0));
    let id = go(&mut state, id, ZoneLocation::Battlefield);
    assert!(!state.object(id).unwrap().status.contains(Status::TAPPED));
}

#[test]
fn an_attachment_is_forgotten_when_the_attached_permanent_leaves() {
    let mut state = state(205);
    let host = permanent(&mut state);
    let gear = permanent(&mut state);
    state.object_mut(gear).unwrap().attached_to = Some(host);
    let gear = go(&mut state, gear, ZoneLocation::Hand(P0));
    assert_eq!(state.object(gear).unwrap().attached_to, None);
}

#[test]
fn the_object_is_listed_in_exactly_one_zone_after_a_move() {
    let mut state = state(206);
    let id = permanent(&mut state);
    let id = go(&mut state, id, ZoneLocation::Graveyard(P0));
    for zone in [
        ZoneLocation::Battlefield,
        ZoneLocation::Hand(P0),
        ZoneLocation::Library(P0),
        ZoneLocation::Exile(P0),
    ] {
        assert!(
            !state.zones.list(zone).contains(&id),
            "not in {zone:?} any more"
        );
    }
    assert!(state.zones.list(ZoneLocation::Graveyard(P0)).contains(&id));
    assert_eq!(state.object(id).unwrap().zone, Zone::Graveyard);
}

#[test]
fn a_card_put_on_top_of_the_library_is_drawn_next() {
    let mut state = state(207);
    let card = state.draw_cards(P0, 1)[0];
    state
        .move_object(
            card,
            ZoneLocation::Library(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    assert_eq!(state.draw_cards(P0, 1), vec![card]);
}

#[test]
fn a_card_put_on_the_bottom_is_not_drawn_next() {
    let mut state = state(208);
    let card = state.draw_cards(P0, 1)[0];
    state
        .move_object(
            card,
            ZoneLocation::Library(P0),
            ZonePosition::Bottom,
            Cause::Effect,
        )
        .unwrap();
    let library = state.zones.list(ZoneLocation::Library(P0));
    assert_eq!(
        library.first(),
        Some(&card),
        "the bottom is the list's start"
    );
    assert_ne!(state.draw_cards(P0, 1), vec![card]);
}

#[test]
fn drawing_more_cards_than_the_library_holds_draws_what_is_there_and_marks_the_try() {
    let mut state = state(209);
    let size = state.zones.list(ZoneLocation::Library(P0)).len();
    let drawn = state.draw_cards(P0, size + 2);
    assert_eq!(drawn.len(), size);
    assert!(state.zones.list(ZoneLocation::Library(P0)).is_empty());
    assert!(
        state.players[0].tried_empty_draw,
        "CR 704.5b reads this at the next check"
    );
    assert!(!state.players[1].tried_empty_draw);
}

#[test]
fn drawing_exactly_the_library_marks_no_empty_draw() {
    let mut state = state(210);
    let size = state.zones.list(ZoneLocation::Library(P0)).len();
    let drawn = state.draw_cards(P0, size);
    assert_eq!(drawn.len(), size);
    assert!(
        !state.players[0].tried_empty_draw,
        "the last card was there to draw"
    );
}

#[test]
fn drawing_zero_cards_changes_nothing() {
    let mut state = state(211);
    let before = state.snapshot_hash();
    assert!(state.draw_cards(P0, 0).is_empty());
    assert_eq!(state.snapshot_hash(), before);
}

#[test]
fn a_draw_takes_the_cards_from_the_top_in_order() {
    let mut state = state(212);
    let library = state.zones.list(ZoneLocation::Library(P0)).clone();
    let drawn = state.draw_cards(P0, 3);
    let expected: Vec<_> = library.iter().rev().take(3).copied().collect();
    assert_eq!(drawn, expected);
}
