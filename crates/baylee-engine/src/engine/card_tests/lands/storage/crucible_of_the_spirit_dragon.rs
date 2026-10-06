//! `cards/lands/storage/crucible_of_the_spirit_dragon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Crucible of the Spirit Dragon prints `{{T}}: Add {{C}}.`, `{{1}}, {{T}}: Put a storage
/// counter on this land.`, and `{{T}}, Remove X storage counters from this land: Add X mana
/// in any combination of colors. Spend this mana only to cast Dragon spells or activate
/// abilities of Dragons.`
///
/// Under `Coverage::Partial`, the ability-spend clause is omitted because no `Filter` can
/// identify the source of an ability on the stack. To pay for ability 1, floating mana is
/// generated from a Forest while Crucible remains untapped. Activating ability 1 spends
/// the floating mana, places a storage counter on the land, and leaves it tapped.
#[test]
fn crucible_of_the_spirit_dragon_banks_storage_counter() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[crucible_of_the_spirit_dragon(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let crucible = on_battlefield(&engine, p0, crucible_of_the_spirit_dragon())
        .expect("crucible on battlefield");
    assert_eq!(counters_on(&engine, crucible, counters::STORAGE), 0);

    // Tap Forest to float {G} without tapping Crucible itself.
    tap_all_mana_but(&mut engine, p0, Some(crucible_of_the_spirit_dragon()));
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);
    assert!(!is_tapped(&engine, crucible));

    activate(&mut engine, p0, crucible_of_the_spirit_dragon(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(counters_on(&engine, crucible, counters::STORAGE), 1);
    assert!(is_tapped(&engine, crucible));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}
