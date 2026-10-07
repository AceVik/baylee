//! `cards/lands/utility/eiganjo_castle.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Eiganjo Castle: "{T}: Add {W}." / "{W}, {T}: Prevent the next 2 damage that would be dealt to target legendary creature this turn."
/// Under `Coverage::Partial`, the activated damage-prevention ability is omitted.
/// The legendary land taps for its base mana ability to add {W} to the mana pool.
#[test]
fn eiganjo_castle_taps_for_white_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(139, forest())
        .battlefield(0, &[eiganjo_castle()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let castle = on_battlefield(&engine, p0, eiganjo_castle()).expect("Castle deployed");
    activate(&mut engine, p0, eiganjo_castle(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::White), 1);
    assert!(is_tapped(&engine, castle));
}
