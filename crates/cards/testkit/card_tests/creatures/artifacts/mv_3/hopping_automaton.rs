//! `cards/creatures/artifacts/mv_3/hopping_automaton.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Hopping Automaton` prints `{{0}}: This creature gets -1/-1 and gains flying until end of turn.`
/// on a 2/2 artifact creature with `Coverage::Implemented`.
/// In this scenario, the zero-mana activated ability is activated on an empty pool.
/// When the ability resolves, the creature becomes a 1/1 and gains `KeywordSet::FLYING`
/// until end of turn as projected by the continuous layer system.
#[test]
fn hopping_automaton_shrinks_by_one_one_and_gains_flying() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[hopping_automaton()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let automaton = on_battlefield(&engine, p0, hopping_automaton()).expect("automaton is seated");
    assert_eq!(pt(&engine, automaton), (2, 2));
    assert!(!keywords(&engine, automaton).contains(KeywordSet::FLYING));

    activate(&mut engine, p0, hopping_automaton(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, automaton), (1, 1));
    assert!(keywords(&engine, automaton).contains(KeywordSet::FLYING));
}
