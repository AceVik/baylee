//! `cards/creatures/mv_1/mwonvuli_ooze.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::MWONVULI_OOZE;

/// `Mwonvuli Ooze` ({G}, Creature — Ooze 1/1, Partial) has cumulative upkeep
/// `{2}`.
#[test]
fn mwonvuli_ooze_sacrificed_when_cumulative_upkeep_declined() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[MWONVULI_OOZE, forest(), forest()])
        .start();
    keep_mulligans(&mut engine);

    assert!(on_battlefield(&engine, p0, MWONVULI_OOZE).is_some());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, MWONVULI_OOZE).is_some());
}
