//! `cards/lands/towers/urza_s_tower.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Urza's Tower: "{T}: Add {C}. If you control an Urza's Mine and an Urza's Power-Plant, add {C}{C}{C} instead."
/// Alone it taps for the one {C}; the other two beside it are
/// `the_urza_lands_make_seven_together_and_one_each_without_the_third_type`.
#[test]
fn urza_s_tower_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(114, forest())
        .battlefield(0, &[urza_s_tower()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let tower = on_battlefield(&engine, p0, urza_s_tower()).expect("Tower deployed");
    activate(&mut engine, p0, urza_s_tower(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, tower));
}
