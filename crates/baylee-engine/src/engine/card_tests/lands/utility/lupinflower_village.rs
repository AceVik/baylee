//! `cards/lands/utility/lupinflower_village.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Lupinflower Village: "{T}: Add {C}." / "{T}: Add {W}. Spend this mana only to cast a creature spell." / "{1}{W}, {T}, Sacrifice this land: Look at the top six cards..."
/// Under `Coverage::Partial`, the `{1}{W}` search ability is omitted because `Effect::LookAtTopPick` carries no type filter.
/// Activating ability 1 adds one restricted white mana spendable only on creature spells to `pool.restricted()`.
#[test]
fn lupinflower_village_adds_restricted_creature_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(228, forest())
        .battlefield(0, &[lupinflower_village()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let village = on_battlefield(&engine, p0, lupinflower_village()).expect("Village deployed");
    activate(&mut engine, p0, lupinflower_village(), 1);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 0);
    assert_eq!(pool.restricted().len(), 1);
    assert_eq!(pool.restricted()[0].amount, 1);
    assert_eq!(pool.restricted()[0].color, ManaColor::White);
    assert!(is_tapped(&engine, village));
}
