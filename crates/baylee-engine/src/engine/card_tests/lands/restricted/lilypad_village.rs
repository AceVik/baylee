//! `cards/lands/restricted/lilypad_village.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Lilypad Village prints `{{T}}: Add {{C}}.`, `{{T}}: Add {{U}}. Spend this mana only
/// to cast a creature spell.`, and `{{U}}, {{T}}: Surveil 2. Activate only if a Bird,
/// Frog, Otter, or Rat entered the battlefield under your control this turn.`
///
/// Under `Coverage::Partial`, the surveil ability is omitted because tracking past
/// creature subtype entries is unsupported. This test activates ability 1, verifying
/// that the restricted blue mana appears in `pool.restricted()` rather than general
/// available mana, and that the land is tapped.
#[test]
fn lilypad_village_adds_restricted_creature_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[lilypad_village()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let village = on_battlefield(&engine, p0, lilypad_village()).expect("village on battlefield");
    activate(&mut engine, p0, lilypad_village(), 1);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 0);
    assert_eq!(pool.restricted().len(), 1);
    assert_eq!(pool.restricted()[0].amount, 1);
    assert_eq!(pool.restricted()[0].color, ManaColor::Blue);
    assert_eq!(pool.total(), 1);
    assert!(is_tapped(&engine, village));
}
