//! `cards/enchantments/mv_1/darkest_hour.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Darkest Hour` (`Coverage::Implemented`):
/// "All creatures are black."
///
/// Verifies that while `Darkest Hour` is on the battlefield, all creatures
/// on both sides of the table have their colors set to black.
#[test]
fn darkest_hour_sets_all_creatures_color_to_black() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1319, forest())
        .battlefield(0, &[swamp(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[darkest_hour()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my elf deployed");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their elf deployed");
    assert_eq!(
        engine
            .state()
            .object(mine)
            .expect("mine exists")
            .characteristics()
            .colors,
        baylee_core::color::ColorSet::from_slice(&[baylee_core::color::Color::Green])
    );

    cast_from_hand(&mut engine, p0, darkest_hour());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, darkest_hour()).is_some());
    assert_eq!(
        engine
            .state()
            .object(mine)
            .expect("mine exists")
            .characteristics()
            .colors,
        baylee_core::color::ColorSet::from_slice(&[baylee_core::color::Color::Black]),
        "my creature is black"
    );
    assert_eq!(
        engine
            .state()
            .object(theirs)
            .expect("theirs exists")
            .characteristics()
            .colors,
        baylee_core::color::ColorSet::from_slice(&[baylee_core::color::Color::Black]),
        "opponent creature is black"
    );
}
