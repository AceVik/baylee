//! `cards/lands/utility/sanctum_of_eternity.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sanctum of Eternity: "{T}: Add {C}." / "{2}, {T}: Return target commander you own from the battlefield to your hand."
/// Under `Coverage::Partial`, the commander-bouncing activation is omitted.
/// Activating Sanctum of Eternity produces {C} and leaves the land tapped.
#[test]
fn sanctum_of_eternity_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(130, forest())
        .battlefield(0, &[sanctum_of_eternity()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = on_battlefield(&engine, p0, sanctum_of_eternity()).expect("Sanctum deployed");
    activate(&mut engine, p0, sanctum_of_eternity(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}
