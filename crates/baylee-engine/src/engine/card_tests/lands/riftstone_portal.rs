//! `cards/lands/riftstone_portal.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Riftstone Portal: "{T}: Add {C}." / "As long as this card is in your graveyard, lands you control have '{T}: Add {G} or {W}.'"
/// Under `Coverage::Partial`, the graveyard static ability granting mana abilities to lands is omitted.
/// The land taps for its base mana ability to add {C} to the mana pool.
#[test]
fn riftstone_portal_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(131, forest())
        .battlefield(0, &[riftstone_portal()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let portal = on_battlefield(&engine, p0, riftstone_portal()).expect("Portal deployed");
    activate(&mut engine, p0, riftstone_portal(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, portal));
}
