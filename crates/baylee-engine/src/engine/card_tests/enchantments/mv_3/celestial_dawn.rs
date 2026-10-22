//! `cards/enchantments/mv_3/celestial_dawn.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::index::{CELESTIAL_DAWN, GRIZZLY_BEARS};
use baylee_core::generated::subtypes;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Celestial Dawn (b4d27fad-9a94-4760-80a6-6a2948109864) — {1}{W}{W} Enchantment:
/// "Lands you control are Plains. Nonland permanents you control are white."
#[test]
fn celestial_dawn_makes_your_lands_plains_and_your_nonland_permanents_white() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), forest(), GRIZZLY_BEARS])
        .battlefield(1, &[forest(), GRIZZLY_BEARS])
        .hand(0, &[CELESTIAL_DAWN])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let p0_forest = on_battlefield(&engine, P0, forest()).unwrap();
    let p0_bears = on_battlefield(&engine, P0, GRIZZLY_BEARS).unwrap();
    let p1_forest = on_battlefield(&engine, P1, forest()).unwrap();
    let p1_bears = on_battlefield(&engine, P1, GRIZZLY_BEARS).unwrap();

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, CELESTIAL_DAWN);
    pass_until(&mut engine, stack_is_empty);

    let p0_forest_chars = engine.state().object(p0_forest).unwrap().characteristics();
    assert!(p0_forest_chars.subtypes.contains(subtypes::land::PLAINS));
    assert!(!p0_forest_chars.subtypes.contains(subtypes::land::FOREST));

    let p0_bears_chars = engine.state().object(p0_bears).unwrap().characteristics();
    assert_eq!(p0_bears_chars.colors, ColorSet::from_slice(&[Color::White]));

    // Opponent's permanents are unaffected.
    assert!(
        engine
            .state()
            .object(p1_forest)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(subtypes::land::FOREST)
    );
    assert_eq!(
        engine
            .state()
            .object(p1_bears)
            .unwrap()
            .characteristics()
            .colors,
        ColorSet::from_slice(&[Color::Green])
    );
}
