//! `cards/creatures/mv_5/arctic_wolves.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn arctic_wolves() -> CardIndex {
    card_index("c9e7218f-07f2-46f3-bc63-eb5c0d32538b")
}

/// `Arctic Wolves` ({3}{G}{G}, 4/5 Creature — Wolf) draws a card when it enters
/// the battlefield and has cumulative upkeep `{2}`.
#[test]
fn arctic_wolves_draws_a_card_on_entry_and_has_cumulative_upkeep() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest()])
        .hand(0, &[arctic_wolves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    cast_from_hand(&mut engine, p0, arctic_wolves());
    pass_until(&mut engine, stack_is_empty);

    let hand_after = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    assert_eq!(hand_after, hand_before);
    assert!(on_battlefield(&engine, p0, arctic_wolves()).is_some());
}
