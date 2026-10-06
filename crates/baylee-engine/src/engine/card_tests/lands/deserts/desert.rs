//! `cards/lands/deserts/desert.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Desert: "{T}: Add {C}." / "{T}: This land deals 1 damage to target attacking creature. Activate only during the end of combat step."
/// Under `Coverage::Partial`, the end-of-combat damage ability is omitted due to timing restrictions.
/// Activating Desert produces {C} and leaves the land tapped.
#[test]
fn desert_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(126, forest()).battlefield(0, &[desert()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = on_battlefield(&engine, p0, desert()).expect("Desert deployed");
    activate(&mut engine, p0, desert(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}
