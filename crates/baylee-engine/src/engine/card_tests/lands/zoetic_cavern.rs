//! `cards/lands/zoetic_cavern.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Zoetic Cavern: "{T}: Add {C}." / "Morph {2}"
/// Under `Coverage::Partial`, casting face-down and turning face up with morph are unsupported.
/// Activating ability 0 adds one colorless mana to the pool and leaves the land tapped.
#[test]
fn zoetic_cavern_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(138, forest())
        .battlefield(0, &[zoetic_cavern()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let cavern = on_battlefield(&engine, p0, zoetic_cavern()).expect("Zoetic Cavern deployed");
    activate(&mut engine, p0, zoetic_cavern(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, cavern));
}
