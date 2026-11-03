//! The plain state-based actions, each on its own: the four ways a player
//! loses (CR 704.5a–c, 903.10a / 704.6c), zero toughness (704.5f), lethal
//! damage and deathtouch (704.5g–h), zero loyalty (704.5i) and the
//! +1/+1 and -1/-1 annihilation (704.5q), with the thresholds checked on
//! both sides.

use super::{RegistryLookup, bare, creature, empty_boards_preset, on_battlefield, run};
use crate::event::LossReason;
use crate::object::CounterKind;
use crate::state::GameState;
use baylee_cards_dsl::KeywordSet;
use baylee_core::types::TypeSet;

fn board(seed: u64) -> GameState {
    GameState::from_preset(&empty_boards_preset(seed), &RegistryLookup).expect("game starts")
}

#[test]
fn one_life_is_still_alive_and_zero_life_loses() {
    let mut state = board(101);
    state.players[1].life = 1;
    let _ = run(&mut state, &RegistryLookup);
    assert!(!state.players[1].has_lost(), "one life is not zero");
    state.players[1].life = 0;
    let _ = run(&mut state, &RegistryLookup);
    assert_eq!(state.players[1].loss, Some(LossReason::Life));
    assert!(!state.players[0].has_lost());
}

#[test]
fn a_negative_life_total_loses_like_zero() {
    let mut state = board(102);
    state.players[0].life = -7;
    let _ = run(&mut state, &RegistryLookup);
    assert_eq!(state.players[0].loss, Some(LossReason::Life));
    assert!(!state.players[1].has_lost());
}

#[test]
fn nine_poison_counters_are_not_ten() {
    let mut state = board(103);
    state.players[0].poison = 9;
    let outcome = run(&mut state, &RegistryLookup);
    assert!(!outcome.changed);
    assert!(!state.players[0].has_lost());
    state.players[0].poison = 11;
    let _ = run(&mut state, &RegistryLookup);
    assert_eq!(state.players[0].loss, Some(LossReason::Poison));
}

#[test]
fn a_draw_from_an_empty_library_loses_at_the_next_check() {
    let mut state = board(104);
    state.players[1].tried_empty_draw = true;
    let _ = run(&mut state, &RegistryLookup);
    assert_eq!(state.players[1].loss, Some(LossReason::EmptyDraw));
    assert!(!state.players[0].has_lost());
}

/// The life total is checked first, so a player who meets two losing
/// conditions at once keeps the first reason, and keeps it on a later pass.
#[test]
fn zero_life_and_ten_poison_together_name_the_life_total() {
    let mut state = board(105);
    state.players[0].life = 0;
    state.players[0].poison = 10;
    state.players[0].tried_empty_draw = true;
    let _ = run(&mut state, &RegistryLookup);
    assert_eq!(state.players[0].loss, Some(LossReason::Life));
    let _ = run(&mut state, &RegistryLookup);
    assert_eq!(state.players[0].loss, Some(LossReason::Life));
}

/// CR 704.3: every player who meets a losing condition in the same check
/// loses at the same time; neither is spared because the other went first.
#[test]
fn both_players_at_zero_lose_in_the_same_check() {
    let mut state = board(106);
    state.players[0].life = 0;
    state.players[1].poison = 10;
    let _ = run(&mut state, &RegistryLookup);
    assert_eq!(state.players[0].loss, Some(LossReason::Life));
    assert_eq!(state.players[1].loss, Some(LossReason::Poison));
}

/// CR 903.10a: twenty-one combat damage from one commander. Twenty from
/// each of three commanders is sixty damage and no loss.
#[test]
fn commander_damage_counts_each_commander_on_its_own() {
    let mut state = board(107);
    let commanders: Vec<_> = (0..3).map(|_| creature(&mut state)).collect();
    state.players[1].commander_damage = commanders.iter().map(|&id| (id, 20)).collect();
    let _ = run(&mut state, &RegistryLookup);
    assert!(
        !state.players[1].has_lost(),
        "three tallies of twenty are not one of twenty-one"
    );
    state.players[1].commander_damage[1].1 = 21;
    let _ = run(&mut state, &RegistryLookup);
    assert_eq!(state.players[1].loss, Some(LossReason::CommanderDamage));
    assert!(!state.players[0].has_lost());
}

/// CR 704.5f is not destruction: indestructible does not keep a creature
/// with zero toughness.
#[test]
fn zero_toughness_kills_an_indestructible_creature() {
    let mut state = board(108);
    let id = creature(&mut state);
    {
        let base = state.object_mut(id).unwrap().base_mut();
        base.toughness = Some(0);
        base.keywords = KeywordSet::INDESTRUCTIBLE;
    }
    state.invalidate_projections();
    assert!(run(&mut state, &RegistryLookup).changed);
    assert!(!on_battlefield(&state, id));
}

#[test]
fn negative_toughness_kills_too() {
    let mut state = board(109);
    let id = creature(&mut state);
    state.object_mut(id).unwrap().base_mut().toughness = Some(-2);
    state.invalidate_projections();
    let _ = run(&mut state, &RegistryLookup);
    assert!(!on_battlefield(&state, id));
}

#[test]
fn damage_one_short_of_toughness_leaves_the_creature_and_lethal_damage_does_not() {
    let mut state = board(110);
    let id = creature(&mut state);
    state.object_mut(id).unwrap().damage = 1;
    assert!(!run(&mut state, &RegistryLookup).changed);
    assert!(on_battlefield(&state, id), "one damage on a 2/2");
    state.object_mut(id).unwrap().damage = 2;
    assert!(run(&mut state, &RegistryLookup).changed);
    assert!(!on_battlefield(&state, id), "two damage on a 2/2");
}

#[test]
fn damage_beyond_toughness_is_lethal_as_well() {
    let mut state = board(111);
    let id = creature(&mut state);
    state.object_mut(id).unwrap().damage = 9;
    let _ = run(&mut state, &RegistryLookup);
    assert!(!on_battlefield(&state, id));
}

#[test]
fn an_indestructible_creature_keeps_any_amount_of_damage() {
    let mut state = board(112);
    let id = creature(&mut state);
    state.object_mut(id).unwrap().base_mut().keywords = KeywordSet::INDESTRUCTIBLE;
    state.invalidate_projections();
    state.object_mut(id).unwrap().damage = 50;
    let _ = run(&mut state, &RegistryLookup);
    assert!(on_battlefield(&state, id));
    assert_eq!(
        state.object(id).unwrap().damage,
        50,
        "the damage stays marked until cleanup"
    );
}

/// CR 704.5h: any damage from a deathtouch source is lethal, however tough
/// the creature; indestructible still keeps it.
#[test]
fn a_deathtouch_mark_kills_a_big_creature_but_not_an_indestructible_one() {
    let mut state = board(113);
    let big = creature(&mut state);
    let sturdy = creature(&mut state);
    state.object_mut(big).unwrap().base_mut().toughness = Some(12);
    state.object_mut(sturdy).unwrap().base_mut().keywords = KeywordSet::INDESTRUCTIBLE;
    state.invalidate_projections();
    for id in [big, sturdy] {
        let obj = state.object_mut(id).unwrap();
        obj.damage = 1;
        obj.deathtouched = true;
    }
    let _ = run(&mut state, &RegistryLookup);
    assert!(!on_battlefield(&state, big));
    assert!(on_battlefield(&state, sturdy));
    assert!(
        !state.object(sturdy).unwrap().deathtouched,
        "the check closes the deathtouch window for the survivor"
    );
}

#[test]
fn a_planeswalker_with_loyalty_stays_and_one_without_goes() {
    let mut state = board(114);
    let kept = bare(&mut state, "Walker one", TypeSet::PLANESWALKER, &[]);
    let gone = bare(&mut state, "Walker two", TypeSet::PLANESWALKER, &[]);
    state
        .object_mut(kept)
        .unwrap()
        .counters
        .set(CounterKind::Loyalty, 1);
    let _ = run(&mut state, &RegistryLookup);
    assert!(on_battlefield(&state, kept));
    assert!(!on_battlefield(&state, gone));
}

/// Loyalty belongs to planeswalkers only: a creature or an artifact with no
/// loyalty counter is not touched by CR 704.5i.
#[test]
fn a_permanent_that_is_no_planeswalker_needs_no_loyalty() {
    let mut state = board(115);
    let beast = creature(&mut state);
    let rock = bare(&mut state, "Rock", TypeSet::ARTIFACT, &[]);
    assert!(!run(&mut state, &RegistryLookup).changed);
    assert!(on_battlefield(&state, beast));
    assert!(on_battlefield(&state, rock));
}

#[test]
fn more_minus_counters_than_plus_counters_leave_only_minus_counters() {
    let mut state = board(116);
    let id = creature(&mut state);
    state.object_mut(id).unwrap().base_mut().toughness = Some(5);
    {
        let obj = state.object_mut(id).unwrap();
        obj.counters.set(CounterKind::P1P1, 1);
        obj.counters.set(CounterKind::M1M1, 3);
    }
    state.invalidate_projections();
    assert!(run(&mut state, &RegistryLookup).changed);
    let obj = state.object(id).unwrap();
    assert_eq!(obj.counters.get(CounterKind::P1P1), 0);
    assert_eq!(obj.counters.get(CounterKind::M1M1), 2);
    assert!(on_battlefield(&state, id), "a 2/5 with net -2/-2 is a 0/3");
}

#[test]
fn equal_counter_piles_cancel_to_nothing() {
    let mut state = board(117);
    let id = creature(&mut state);
    {
        let obj = state.object_mut(id).unwrap();
        obj.counters.set(CounterKind::P1P1, 4);
        obj.counters.set(CounterKind::M1M1, 4);
    }
    state.invalidate_projections();
    assert!(run(&mut state, &RegistryLookup).changed);
    let obj = state.object(id).unwrap();
    assert_eq!(obj.counters.get(CounterKind::P1P1), 0);
    assert_eq!(obj.counters.get(CounterKind::M1M1), 0);
    assert!(on_battlefield(&state, id));
}

#[test]
fn plus_counters_alone_are_left_alone() {
    let mut state = board(118);
    let id = creature(&mut state);
    state
        .object_mut(id)
        .unwrap()
        .counters
        .set(CounterKind::P1P1, 3);
    state.invalidate_projections();
    assert!(!run(&mut state, &RegistryLookup).changed);
    assert_eq!(state.object(id).unwrap().counters.get(CounterKind::P1P1), 3);
}

/// A settled board is a fixpoint: a second pass straight after a pass that
/// changed something finds nothing more to do.
#[test]
fn a_second_pass_after_a_death_changes_nothing() {
    let mut state = board(119);
    let dying = creature(&mut state);
    let _alive = creature(&mut state);
    state.object_mut(dying).unwrap().damage = 2;
    assert!(run(&mut state, &RegistryLookup).changed);
    assert!(!run(&mut state, &RegistryLookup).changed);
}

#[test]
fn a_loss_is_recorded_once_and_does_not_report_a_change_again() {
    let mut state = board(120);
    state.players[1].life = 0;
    assert!(run(&mut state, &RegistryLookup).changed);
    assert!(state.players[1].has_lost());
    assert!(!run(&mut state, &RegistryLookup).changed);
    assert_eq!(state.players[1].loss, Some(LossReason::Life));
}
