//! `cards/creatures/mv_2/drudge_skeletons.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Drudge Skeletons — vanilla 1/1 plus `{B}`: Regenerate this creature.
#[test]
fn drudge_skeletons_regenerates_for_b() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[drudge_skeletons(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let skeleton = on_battlefield(&engine, p0, drudge_skeletons()).expect("seated");
    assert_eq!(pt(&engine, skeleton), (1, 1));
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, drudge_skeletons(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine
            .state()
            .object(skeleton)
            .unwrap()
            .regeneration_shields,
        1,
        "the floating {{B}} bought a shield"
    );
}
