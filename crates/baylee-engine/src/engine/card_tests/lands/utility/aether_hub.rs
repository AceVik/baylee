//! `cards/lands/utility/aether_hub.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Aether Hub: "When this land enters, you get {E}." / "{T}: Add {C}." / "{T}, Pay {E}: Add one mana of any color."
/// Under `Coverage::Partial`, energy counters on players and paying energy are omitted.
/// Activating the land's implemented ability adds {C} to the mana pool and leaves it tapped.
#[test]
fn aether_hub_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(104, forest())
        .battlefield(0, &[aether_hub()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hub = on_battlefield(&engine, p0, aether_hub()).expect("Aether Hub deployed");
    activate(&mut engine, p0, aether_hub(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, hub));
}
