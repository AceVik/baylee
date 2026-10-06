//! `cards/lands/eldrazi_temple.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Eldrazi Temple: "{T}: Add {C}." / "{T}: Add {C}{C}. Spend this mana only to cast colorless Eldrazi spells or activate abilities of colorless Eldrazi."
/// Under `Coverage::Partial`, activating ability 1 adds two restricted colorless mana for colorless Eldrazi spells.
/// The restricted mana appears in `pool.restricted()` rather than general available colorless mana.
#[test]
fn eldrazi_temple_adds_restricted_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(105, forest())
        .battlefield(0, &[eldrazi_temple()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let temple = on_battlefield(&engine, p0, eldrazi_temple()).expect("Eldrazi Temple deployed");
    activate(&mut engine, p0, eldrazi_temple(), 1);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 0);
    assert_eq!(pool.restricted().len(), 1);
    assert_eq!(pool.restricted()[0].amount, 2);
    assert_eq!(pool.restricted()[0].color, ManaColor::Colorless);
    assert!(is_tapped(&engine, temple));
}
