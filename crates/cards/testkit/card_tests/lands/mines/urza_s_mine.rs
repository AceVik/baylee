//! `cards/lands/mines/urza_s_mine.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Urza's Mine: "{T}: Add {C}. If you control an Urza's Power-Plant and an Urza's Tower, add {C}{C} instead."
/// Alone it taps for the one {C}; the other two beside it are
/// `the_urza_lands_make_seven_together_and_one_each_without_the_third_type`.
#[test]
fn urza_s_mine_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(110, forest())
        .battlefield(0, &[urza_s_mine()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, urza_s_mine()).expect("Mine deployed");
    activate(&mut engine, p0, urza_s_mine(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, mine));
}
