//! `cards/creatures/mv_3/uthden_troll.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Uthden Troll — vanilla 2/2 plus `{R}`: Regenerate this creature.
#[test]
fn uthden_troll_regenerates_for_r() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[uthden_troll(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let troll = on_battlefield(&engine, p0, uthden_troll()).expect("seated");
    assert_eq!(pt(&engine, troll), (2, 2));
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, uthden_troll(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(troll).unwrap().regeneration_shields,
        1
    );
}
