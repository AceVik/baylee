//! `cards/creatures/artifacts/mv_5/igneous_golem.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Igneous Golem` prints `{{2}}: This creature gains trample until end of turn.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls the 3/4 artifact creature and two Forests.
/// Activating the ability off two floating mana places it on the stack.
/// Upon resolution, `Igneous Golem` gains `KeywordSet::TRAMPLE` until end of turn.
#[test]
fn igneous_golem_gains_trample_until_end_of_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[igneous_golem(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let golem = on_battlefield(&engine, p0, igneous_golem()).expect("igneous golem is seated");
    assert_eq!(pt(&engine, golem), (3, 4));
    assert!(
        !keywords(&engine, golem).contains(KeywordSet::TRAMPLE),
        "starts without trample"
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);

    activate(&mut engine, p0, igneous_golem(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, golem).contains(KeywordSet::TRAMPLE),
        "`Igneous Golem` gains trample upon resolution"
    );
    assert_eq!(pt(&engine, golem), (3, 4));
}
