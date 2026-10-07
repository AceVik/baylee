//! `cards/artifacts/mv_2/iron_lance.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Iron Lance` prints `{{3}}, {{T}}: Target creature gains first strike until end of turn.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Iron Lance`, three copies of `forest()`, and a `llanowar_elves()`.
/// Paying three mana to activate the lance targets the elf creature, tapping `Iron Lance`
/// and granting `KeywordSet::FIRST_STRIKE` to the target creature until end of turn upon resolution.
#[test]
fn iron_lance_grants_first_strike_to_target_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), iron_lance(), llanowar_elves()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("elves on battlefield");
    let lance = on_battlefield(&engine, p0, iron_lance()).expect("lance on battlefield");
    assert!(!keywords(&engine, elves).contains(KeywordSet::FIRST_STRIKE));

    tap_mana_where(&mut engine, p0, |id| id != lance && id != elves);
    assert_eq!(engine.state().players[0].mana_pool.total(), 3);

    activate(&mut engine, p0, iron_lance(), 0);
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
    assert!(is_tapped(&engine, lance), "`Iron Lance` is tapped");
}
