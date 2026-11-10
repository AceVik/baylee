//! `cards/lands/corrupted_crossroads.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Corrupted Crossroads: "{T}: Add {C}." / "{T}, Pay 1 life: Add one mana of any color. Spend this mana only to cast a spell with devoid."
/// Under `Coverage::Partial`, the devoid mana restriction is unsupported and that ability is omitted.
/// Activating ability 0 produces one colorless mana and taps the land.
#[test]
fn corrupted_crossroads_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(103, forest())
        .battlefield(0, &[corrupted_crossroads()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let crossroads =
        on_battlefield(&engine, p0, corrupted_crossroads()).expect("Crossroads deployed");
    activate(&mut engine, p0, corrupted_crossroads(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, crossroads));
}
