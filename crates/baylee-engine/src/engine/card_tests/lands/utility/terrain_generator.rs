//! `cards/lands/utility/terrain_generator.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Terrain Generator: "{T}: Add {C}." / "{2}, {T}: You may put a basic land card from your hand onto the battlefield tapped."
/// Under `Coverage::Partial`, the hand-to-battlefield basic land drop ability is omitted.
/// The land taps for its base mana ability to add {C} to the mana pool.
#[test]
fn terrain_generator_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(122, forest())
        .battlefield(0, &[terrain_generator()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let generator = on_battlefield(&engine, p0, terrain_generator()).expect("Generator deployed");
    activate(&mut engine, p0, terrain_generator(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, generator));
}
