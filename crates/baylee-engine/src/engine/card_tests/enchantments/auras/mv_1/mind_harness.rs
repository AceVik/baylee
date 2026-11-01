//! `cards/enchantments/auras/mv_1/mind_harness.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "bba30360-c54a-4906-8b2e-84bb1209c43d"
fn mind_harness() -> CardIndex {
    card_index("bba30360-c54a-4906-8b2e-84bb1209c43d")
}

#[test]
fn mind_harness_gains_control_of_red_or_green_creature() {
    let mut engine = lk::game(SEED, &[], &[index::GRIZZLY_BEARS], &[mind_harness()], &[]);
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    lk::float(&mut engine, P0, &[(ManaColor::Blue, 1)]);
    cast_with_floating(&mut engine, P0, mind_harness());
    lk::aim(&mut engine, &[bears], &[]);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(bears).unwrap().controller, P0);
}

fn age_counters(engine: &Engine<RegistryLookup>, id: ObjectId) -> u16 {
    engine
        .state()
        .object(id)
        .map_or(0, |o| o.counters.get(baylee_cards_dsl::counters::AGE))
}

/// "Enchant red or green creature": a blue and a white creature are not on the
/// target menu, the green one is.
#[test]
fn mind_harness_offers_only_red_or_green_creatures() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[
            index::GRIZZLY_BEARS,
            index::STORM_CROW,
            index::SAVANNAH_LIONS,
        ],
        &[mind_harness()],
        &[],
    );
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    lk::float(&mut engine, P0, &[(ManaColor::Blue, 1)]);
    cast_with_floating(&mut engine, P0, mind_harness());
    let menu = aim_at(&mut engine, P0, bears);
    assert_eq!(
        menu,
        vec![bears],
        "Storm Crow is blue and Savannah Lions white"
    );
}

/// "Cumulative upkeep {1}": the first upkeep adds an age counter and asks {1};
/// declining sacrifices the Aura and the creature goes back.
#[test]
fn mind_harness_sacrificed_when_the_upkeep_cost_is_declined_returns_the_creature() {
    let mut engine = lk::game(SEED, &[], &[index::GRIZZLY_BEARS], &[mind_harness()], &[]);
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    lk::float(&mut engine, P0, &[(ManaColor::Blue, 1)]);
    cast_with_floating(&mut engine, P0, mind_harness());
    lk::aim(&mut engine, &[bears], &[]);
    lk::settle(&mut engine);
    assert_eq!(engine.state().object(bears).unwrap().controller, P0);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    assert_eq!(
        (engine.state().turn.number, engine.state().turn.active),
        (3, P0),
        "cumulative upkeep asks in its controller's own upkeep"
    );
    let harness = lk::obj(&engine, P0, mind_harness());
    assert_eq!(age_counters(&engine, harness), 1);

    lk::yes_no(&mut engine, false);
    lk::settle(&mut engine);
    assert!(
        in_graveyard(&engine, P0, mind_harness()).is_some(),
        "declining cumulative upkeep sacrifices Mind Harness"
    );
    assert_eq!(
        engine.state().object(bears).unwrap().controller,
        P1,
        "the creature goes back to its owner"
    );
}

/// Paying keeps the Aura and the creature; the next upkeep has two age
/// counters and costs {2}.
#[test]
fn mind_harness_paid_upkeep_keeps_control_and_costs_one_more_each_time() {
    let mut engine = lk::game(SEED, &[], &[index::GRIZZLY_BEARS], &[mind_harness()], &[]);
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    lk::float(&mut engine, P0, &[(ManaColor::Blue, 1)]);
    cast_with_floating(&mut engine, P0, mind_harness());
    lk::aim(&mut engine, &[bears], &[]);
    lk::settle(&mut engine);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    lk::float(&mut engine, P0, &[(ManaColor::Colorless, 1)]);
    lk::yes_no(&mut engine, true);
    lk::settle(&mut engine);
    let harness = lk::obj(&engine, P0, mind_harness());
    assert_eq!(age_counters(&engine, harness), 1);
    assert_eq!(lk::pool_total(&engine, P0), 0, "{{1}} was paid");
    assert_eq!(engine.state().object(bears).unwrap().controller, P0);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    assert_eq!(
        (engine.state().turn.number, engine.state().turn.active),
        (5, P0)
    );
    assert_eq!(age_counters(&engine, harness), 2);
    lk::float(&mut engine, P0, &[(ManaColor::Colorless, 2)]);
    lk::yes_no(&mut engine, true);
    lk::settle(&mut engine);
    assert_eq!(
        lk::pool_total(&engine, P0),
        0,
        "two age counters cost {{2}}"
    );
    assert_eq!(engine.state().object(bears).unwrap().controller, P0);
}
