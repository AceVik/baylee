//! `cards/lands/utility/haunted_fengraf.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Haunted Fengraf: "{T}: Add {C}." / "{3}, {T}, Sacrifice this land: Return a creature card at random from your graveyard to your hand."
/// Under `Coverage::Partial`, the random graveyard return ability is omitted.
/// Activating the land's implemented ability adds {C} to the mana pool and taps it.
#[test]
fn haunted_fengraf_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(112, forest())
        .battlefield(0, &[haunted_fengraf()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = on_battlefield(&engine, p0, haunted_fengraf()).expect("Haunted Fengraf deployed");
    activate(&mut engine, p0, haunted_fengraf(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}
