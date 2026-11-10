//! `cards/creatures/artifacts/mv_5/henge_guardian.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Henge Guardian` prints `{{2}}: This creature gains trample until end of turn.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls the 3/4 artifact creature and two Forests.
/// Tapping the Forests supplies two mana to activate the ability.
/// Upon resolution, the creature gains `KeywordSet::TRAMPLE` until end of turn while maintaining its 3/4 stats.
#[test]
fn henge_guardian_gains_trample_until_end_of_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[henge_guardian(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let guardian = on_battlefield(&engine, p0, henge_guardian()).expect("guardian is seated");
    assert_eq!(pt(&engine, guardian), (3, 4));
    assert!(
        !keywords(&engine, guardian).contains(KeywordSet::TRAMPLE),
        "`Henge Guardian` does not have trample initially"
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);

    activate(&mut engine, p0, henge_guardian(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, guardian).contains(KeywordSet::TRAMPLE),
        "`Henge Guardian` gains trample after activation resolves"
    );
    assert_eq!(pt(&engine, guardian), (3, 4));
}
