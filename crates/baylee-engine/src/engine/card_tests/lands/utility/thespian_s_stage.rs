//! `cards/lands/utility/thespian_s_stage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Thespian's Stage: "{T}: Add {C}." / "{2}, {T}: This land becomes a copy of target land, except it has this ability."
/// Under `Coverage::Partial`, the target-copying activated ability is omitted.
/// The land taps for its base mana ability to add {C} to the mana pool.
#[test]
fn thespian_s_stage_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(112, forest())
        .battlefield(0, &[thespian_s_stage()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let stage = on_battlefield(&engine, p0, thespian_s_stage()).expect("Stage deployed");
    activate(&mut engine, p0, thespian_s_stage(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, stage));
}
