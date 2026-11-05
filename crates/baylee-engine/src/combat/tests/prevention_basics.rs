//! Prevention shields against combat damage, the counted kind and the
//! fog kind, each read on what it protects and on what it does not
//! (CR 615.1a, 615.7): "prevent the next N damage" is used up as it
//! prevents, a fog is not, and a shield on one creature leaves the player
//! and the other creatures alone.

use super::*;
use crate::prevention::{Shield, ShieldKind, Shielded};

fn life(state: &GameState, seat: PlayerId) -> i32 {
    state.players[usize::from(seat.get())].life
}

fn shield(state: &mut GameState, protects: Shielded, kind: ShieldKind) {
    state.shields.push(Shield {
        protects,
        kind,
        controller: P1,
    });
}

fn version(state: &GameState, id: ObjectId) -> u32 {
    state.object(id).unwrap().version
}

#[test]
fn a_counted_shield_on_a_player_prevents_that_much_and_the_rest_gets_through() {
    let mut state = empty_state();
    let bear = creature(&mut state, P0, 5, 5, KeywordSet::EMPTY);
    attack(&mut state, bear, P1);
    shield(&mut state, Shielded::Player(P1), ShieldKind::Next(3));
    deal_combat_damage(&mut state, false);
    assert_eq!(life(&state, P1), 18, "five dealt, three prevented");
    assert!(state.shields.is_empty(), "the shield is spent");
}

#[test]
fn a_counted_shield_bigger_than_the_damage_keeps_what_is_left() {
    let mut state = empty_state();
    let bear = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
    attack(&mut state, bear, P1);
    shield(&mut state, Shielded::Player(P1), ShieldKind::Next(5));
    deal_combat_damage(&mut state, false);
    assert_eq!(life(&state, P1), 20);
    assert_eq!(state.shields.len(), 1, "three points of shield are left");
    let left = state.shields.identified().next().map(|(_, s, _)| s.kind);
    assert_eq!(left, Some(ShieldKind::Next(3)));
}

#[test]
fn a_shield_on_one_player_does_not_cover_another() {
    let mut state = seated(3);
    let p2 = PlayerId::new(2);
    let one = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
    let two = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
    attack(&mut state, one, P1);
    attack(&mut state, two, p2);
    shield(&mut state, Shielded::Player(P1), ShieldKind::AllCombat);
    deal_combat_damage(&mut state, false);
    assert_eq!(life(&state, P1), 20);
    assert_eq!(life(&state, p2), 18);
}

#[test]
fn a_shield_on_a_creature_saves_it_and_not_its_controller() {
    let mut state = empty_state();
    let attacker = creature(&mut state, P0, 3, 3, KeywordSet::EMPTY);
    let blocker = creature(&mut state, P1, 1, 3, KeywordSet::EMPTY);
    let unblocked = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
    attack(&mut state, attacker, P1);
    attack(&mut state, unblocked, P1);
    block(&mut state, blocker, attacker);
    let v = version(&state, blocker);
    shield(
        &mut state,
        Shielded::Object(blocker, v),
        ShieldKind::AllCombat,
    );
    deal_combat_damage(&mut state, false);
    assert_eq!(damage(&state, blocker), 0);
    assert_eq!(
        damage(&state, attacker),
        1,
        "the shielded blocker still strikes"
    );
    assert_eq!(life(&state, P1), 18, "the player is not covered");
}

#[test]
fn a_fog_lasts_for_both_damage_steps() {
    let mut state = empty_state();
    let striker = creature(&mut state, P0, 2, 2, KeywordSet::DOUBLE_STRIKE);
    attack(&mut state, striker, P1);
    shield(&mut state, Shielded::Everything, ShieldKind::AllCombat);
    deal_combat_damage(&mut state, true);
    deal_combat_damage(&mut state, false);
    assert_eq!(life(&state, P1), 20);
}

#[test]
fn a_counted_shield_spans_the_two_damage_steps() {
    let mut state = empty_state();
    let striker = creature(&mut state, P0, 2, 2, KeywordSet::DOUBLE_STRIKE);
    attack(&mut state, striker, P1);
    shield(&mut state, Shielded::Player(P1), ShieldKind::Next(3));
    deal_combat_damage(&mut state, true);
    assert_eq!(life(&state, P1), 20, "two of three prevented");
    deal_combat_damage(&mut state, false);
    assert_eq!(
        life(&state, P1),
        19,
        "the last point of shield takes one of two"
    );
    assert!(state.shields.is_empty());
}

#[test]
fn prevented_damage_gains_a_lifelinker_nothing() {
    let mut state = empty_state();
    let lifelinker = creature(&mut state, P0, 4, 4, KeywordSet::LIFELINK);
    attack(&mut state, lifelinker, P1);
    shield(&mut state, Shielded::Player(P1), ShieldKind::Next(3));
    deal_combat_damage(&mut state, false);
    assert_eq!(life(&state, P1), 19);
    assert_eq!(
        life(&state, P0),
        21,
        "lifelink counts only the damage dealt"
    );
}
