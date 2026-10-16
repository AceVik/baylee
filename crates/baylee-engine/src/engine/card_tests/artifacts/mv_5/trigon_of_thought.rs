//! `cards/artifacts/mv_5/trigon_of_thought.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn trigon_of_thought() -> CardIndex {
    card_index("8a179d68-c9e6-4f46-99bf-11b2a2708990")
}

/// "This artifact enters with three charge counters on it."
#[test]
fn it_enters_with_three_charge_counters() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 5])
        .hand(0, &[trigon_of_thought()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, trigon_of_thought());
    pass_until(&mut engine, stack_is_empty);
    let trigon = on_battlefield(&engine, p0, trigon_of_thought()).unwrap();
    assert_eq!(counters_on(&engine, trigon, CounterKind::Charge), 3);
}

/// "{U}{U}, {T}: Put a charge counter on this artifact." Only blue pays it.
#[test]
fn two_blue_mana_add_a_charge_counter() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[trigon_of_thought(), island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let trigon = on_battlefield(&engine, p0, trigon_of_thought()).unwrap();
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, trigon_of_thought(), 0);
    assert!(is_tapped(&engine, trigon));
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, trigon, CounterKind::Charge), 1);
}

/// Green cannot pay for the {U}{U}.
#[test]
fn green_mana_does_not_pay_for_the_blue_line() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[trigon_of_thought(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let trigon = on_battlefield(&engine, p0, trigon_of_thought()).unwrap();
    tap_all_mana(&mut engine, p0);
    assert!(!priority_offer(&engine).abilities.contains(&(trigon, 0)));
}

/// "{2}, {T}, Remove a charge counter from this artifact: Draw a card."
#[test]
fn a_counter_and_two_mana_draw_a_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[trigon_of_thought(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let trigon = on_battlefield(&engine, p0, trigon_of_thought()).unwrap();
    set_counters(&mut engine, trigon, CounterKind::Charge, 3);
    let hand = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, trigon_of_thought(), 1);
    assert!(is_tapped(&engine, trigon));
    assert_eq!(
        counters_on(&engine, trigon, CounterKind::Charge),
        2,
        "the counter is a cost"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand + 1
    );
}
