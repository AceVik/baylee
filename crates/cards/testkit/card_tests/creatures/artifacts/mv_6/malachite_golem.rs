//! `cards/creatures/artifacts/mv_6/malachite_golem.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Malachite Golem` prints `{{1}}{{G}}: This creature gains trample until end of turn.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls the 5/3 artifact creature and two Forests.
/// Tapping the Forests supplies the required mana to activate the ability.
/// Upon resolution, `Malachite Golem` gains `KeywordSet::TRAMPLE` until end of turn.
#[test]
fn malachite_golem_gains_trample_until_end_of_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[malachite_golem(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let golem = on_battlefield(&engine, p0, malachite_golem()).expect("malachite golem seated");
    assert_eq!(pt(&engine, golem), (5, 3));
    assert!(
        !keywords(&engine, golem).contains(KeywordSet::TRAMPLE),
        "starts without trample"
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);

    activate(&mut engine, p0, malachite_golem(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, golem).contains(KeywordSet::TRAMPLE),
        "`Malachite Golem` gains trample upon resolution"
    );
    assert_eq!(pt(&engine, golem), (5, 3));
}
