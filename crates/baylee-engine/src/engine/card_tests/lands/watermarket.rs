//! `cards/lands/watermarket.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Watermarket: "{T}: Add {C}{C}. Spend this mana only to cast spells with watermarks."
/// Under `Coverage::Partial`, watermark spend restrictions are unsupported and the mana is made unrestricted.
/// Activating Watermarket's printed mana ability adds two colorless mana to the pool and taps the land.
#[test]
fn watermarket_taps_for_two_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(76, forest())
        .battlefield(0, &[watermarket()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wm = on_battlefield(&engine, p0, watermarket()).expect("Watermarket deployed");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        0
    );

    activate(&mut engine, p0, watermarket(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        2,
        "Watermarket produces {{C}}{{C}}"
    );
    assert!(is_tapped(&engine, wm));
}
