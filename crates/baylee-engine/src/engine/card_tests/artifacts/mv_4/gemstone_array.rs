//! `cards/artifacts/mv_4/gemstone_array.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn gemstone_array() -> CardIndex {
    card_index("91303465-4b98-4ecc-82f4-cfd2f9a0f8a7")
}

/// "{2}: Put a charge counter on this artifact." Each payment is a counter,
/// with no {T} in the way of paying twice.
#[test]
fn two_mana_buys_a_charge_counter_as_often_as_it_is_paid() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[gemstone_array(), forest(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let array = on_battlefield(&engine, p0, gemstone_array()).unwrap();

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, gemstone_array(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, array, CounterKind::Charge), 1);
    activate(&mut engine, p0, gemstone_array(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, array, CounterKind::Charge), 2);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0, "four paid");
}

/// "Remove a charge counter from this artifact: Add one mana of any color."
/// It is a mana ability (no stack), needs a counter, and takes exactly one.
#[test]
fn a_charge_counter_is_spent_for_one_mana_of_any_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[gemstone_array()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let array = on_battlefield(&engine, p0, gemstone_array()).unwrap();

    assert!(
        !priority_offer(&engine).mana_abilities.contains(&array),
        "no counter, no mana"
    );
    set_counters(&mut engine, array, CounterKind::Charge, 2);
    assert!(priority_offer(&engine).mana_abilities.contains(&array));
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: array })
        .expect("offered");
    while matches!(engine.pending(), Pending::ChooseColor { .. }) {
        engine
            .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
            .expect("a listed color");
    }
    assert_eq!(counters_on(&engine, array, CounterKind::Charge), 1);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1, "the chosen color");
    assert_eq!(pool.total(), 1, "and only one mana");
    assert!(!is_tapped(&engine, array), "the cost has no {{T}}");
    assert!(stack_is_empty(&engine), "a mana ability, CR 605.3b");
}
