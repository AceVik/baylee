//! `cards/creatures/mv_2/revered_unicorn.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::REVERED_UNICORN;

/// `Revered Unicorn` ({1}{W}, Creature — Unicorn 2/3) has cumulative upkeep
/// `{1}` and gains life equal to the number of age counters on it when it
/// leaves the battlefield.
#[test]
fn revered_unicorn_gains_life_for_age_counters_when_leaving_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[REVERED_UNICORN, plains()])
        .start();
    keep_mulligans(&mut engine);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, REVERED_UNICORN).is_some());
    assert_eq!(engine.state().players[0].life, 21);
}
