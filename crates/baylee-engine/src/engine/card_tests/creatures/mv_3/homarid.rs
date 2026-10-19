//! `cards/creatures/mv_3/homarid.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::Step;
use baylee_cards_dsl::counters;

const P0: PlayerId = PlayerId::new(0);

fn homarid() -> CardIndex {
    card_index("5d02b0d9-cc53-4967-84bb-8b7f5c489b5c")
}

/// Turn one, Homarid cast off three Islands and resolved.
fn with_homarid() -> (Engine<RegistryLookup>, ObjectId) {
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[homarid()])
        .battlefield(0, &[island(), island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    cast_from_hand(&mut engine, P0, homarid());
    pass_until(&mut engine, stack_is_empty);
    let id = on_battlefield(&engine, P0, homarid()).expect("Homarid resolved");
    (engine, id)
}

fn add_tide(engine: &mut Engine<RegistryLookup>, id: ObjectId) {
    crate::replacement::put_counters(&mut engine.state, id, counters::TIDE, 1);
    engine.sync_static_effects();
    engine.state.refresh_characteristics();
}

fn next_own_upkeep(engine: &mut Engine<RegistryLookup>) {
    pass_until(engine, |e| {
        e.state().turn.active == P0 && e.state().turn.step == Step::Upkeep
    });
    pass_until(engine, stack_is_empty);
}

/// Homarid (5d02b0d9-cc53-4967-84bb-8b7f5c489b5c) — "This creature enters
/// with a tide counter on it." and "As long as there is exactly one tide
/// counter on this creature, it gets -1/-1." (a 2/2 printed)
#[test]
fn homarid_enters_with_one_tide_counter_and_is_a_one_one() {
    let (engine, id) = with_homarid();
    assert_eq!(counters_on(&engine, id, counters::TIDE), 1);
    assert_eq!(pt(&engine, id), (1, 1), "exactly one counter: -1/-1");
}

/// "At the beginning of your upkeep, put a tide counter on this creature."
#[test]
fn homarid_gains_a_tide_counter_each_of_your_upkeeps() {
    let (mut engine, id) = with_homarid();
    next_own_upkeep(&mut engine);
    assert_eq!(counters_on(&engine, id, counters::TIDE), 2);
    assert_eq!(pt(&engine, id), (2, 2), "two counters: no modifier");
}

/// "As long as there are exactly three tide counters ... +1/+1."
#[test]
fn homarid_with_exactly_three_counters_is_a_three_three() {
    let (mut engine, id) = with_homarid();
    add_tide(&mut engine, id);
    assert_eq!(pt(&engine, id), (2, 2));
    add_tide(&mut engine, id);
    assert_eq!(counters_on(&engine, id, counters::TIDE), 3);
    assert_eq!(pt(&engine, id), (3, 3), "exactly three: +1/+1");
}

/// "Whenever there are four or more tide counters on this creature, remove
/// all tide counters from it." The fourth arrives with the upkeep and goes.
#[test]
fn homarid_loses_all_its_tide_counters_at_four() {
    let (mut engine, id) = with_homarid();
    add_tide(&mut engine, id);
    add_tide(&mut engine, id);
    assert_eq!(counters_on(&engine, id, counters::TIDE), 3);
    next_own_upkeep(&mut engine);
    assert_eq!(
        counters_on(&engine, id, counters::TIDE),
        0,
        "the fourth counter set the state trigger off and all were removed"
    );
    assert_eq!(pt(&engine, id), (2, 2), "no counters: a plain 2/2");
}

/// The state trigger also fires on more than four.
#[test]
fn homarid_with_five_counters_loses_them_all() {
    let (mut engine, id) = with_homarid();
    for _ in 0..4 {
        add_tide(&mut engine, id);
    }
    assert_eq!(counters_on(&engine, id, counters::TIDE), 5);
    pass_until(&mut engine, |e| counters_on(e, id, counters::TIDE) == 0);
    assert_eq!(counters_on(&engine, id, counters::TIDE), 0);
}
