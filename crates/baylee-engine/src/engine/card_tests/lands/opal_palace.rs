//! `cards/lands/opal_palace.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The printed mana or land-face ability is exercised through player actions.
#[test]
fn opal_palace_produces_commander_identity_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(319, forest())
        .commander(0, &[sheoldred_the_apocalypse()])
        .battlefield(0, &[opal_palace(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let palace = on_battlefield(&engine, p0, opal_palace()).expect("palace deployed");
    tap_mana_except(&mut engine, p0, palace);
    activate(&mut engine, p0, opal_palace(), 1);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1);
    assert!(is_tapped(&engine, palace));
}
