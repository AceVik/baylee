//! `cards/creatures/mv_1/will_o_the_wisp.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Will-o'-the-Wisp — Flying; `{B}`: Regenerate this creature.
#[test]
fn will_o_the_wisp_flies_and_regenerates_for_b() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[will_o_the_wisp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let wisp = on_battlefield(&engine, p0, will_o_the_wisp()).expect("seated");
    assert_eq!(pt(&engine, wisp), (0, 1));
    assert!(keywords(&engine, wisp).contains(KeywordSet::FLYING));
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, will_o_the_wisp(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(wisp).unwrap().regeneration_shields, 1);
}
