//! `cards/lands/r_d_s_secret_lair.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// R&D's Secret Lair: "Play cards as written. Ignore all errata." / "{T}: Add {C}."
/// Under `Coverage::Partial`, the un-set errata-ignoring rule is omitted.
/// The legendary land taps for its base mana ability to add {C} to the mana pool.
#[test]
fn r_d_s_secret_lair_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(129, forest())
        .battlefield(0, &[r_d_s_secret_lair()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lair = on_battlefield(&engine, p0, r_d_s_secret_lair()).expect("Lair deployed");
    activate(&mut engine, p0, r_d_s_secret_lair(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, lair));
}
