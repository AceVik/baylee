//! `cards/creatures/artifacts/mv_7/flowstone_thopter.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Flowstone Thopter` prints `{{1}}: This creature gets +1/-1 and gains flying until end of turn.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls the 4/4 artifact creature and two Forests.
/// Activating the ability shifts its power and toughness to 5/3 while granting `KeywordSet::FLYING`.
/// A second activation further adjusts its power/toughness to 6/2 until end of turn.
#[test]
fn flowstone_thopter_gains_flying_and_shifts_stats() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[flowstone_thopter(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let thopter = on_battlefield(&engine, p0, flowstone_thopter()).expect("thopter seated");
    assert_eq!(pt(&engine, thopter), (4, 4));
    assert!(
        !keywords(&engine, thopter).contains(KeywordSet::FLYING),
        "starts without flying"
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);

    activate(&mut engine, p0, flowstone_thopter(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, thopter), (5, 3), "first activation yields 5/3");
    assert!(
        keywords(&engine, thopter).contains(KeywordSet::FLYING),
        "gained flying"
    );

    activate(&mut engine, p0, flowstone_thopter(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, thopter), (6, 2), "second activation yields 6/2");
    assert!(
        keywords(&engine, thopter).contains(KeywordSet::FLYING),
        "retains flying"
    );
}
