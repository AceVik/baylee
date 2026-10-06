//! `cards/artifacts/mv_2/fyndhorn_bow.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Fyndhorn Bow` prints `{{3}}, {{T}}: Target creature gains first strike until end of turn.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Fyndhorn Bow`, three copies of `forest()`, and a `llanowar_elves()`.
/// Spending three mana from the forests pays to activate the bow targeting the elves,
/// granting `KeywordSet::FIRST_STRIKE` to the target creature until end of turn upon resolution.
#[test]
fn fyndhorn_bow_grants_first_strike_to_target_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                fyndhorn_bow(),
                llanowar_elves(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("elves on battlefield");
    let bow = on_battlefield(&engine, p0, fyndhorn_bow()).expect("bow on battlefield");
    assert!(!keywords(&engine, elves).contains(KeywordSet::FIRST_STRIKE));

    tap_mana_where(&mut engine, p0, |id| id != bow && id != elves);
    assert_eq!(engine.state().players[0].mana_pool.total(), 3);

    activate(&mut engine, p0, fyndhorn_bow(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets prompt, got {:?}", engine.pending());
    };
    assert!(options.contains(&elves), "creature is a legal target");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .expect("targeted elves");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, elves).contains(KeywordSet::FIRST_STRIKE),
        "target creature gained first strike"
    );
    assert!(is_tapped(&engine, bow), "`Fyndhorn Bow` is tapped");
}
