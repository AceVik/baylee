//! `cards/enchantments/mv_1/ajani_s_welcome.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn ajani_s_welcome() -> CardIndex {
    card_index("4a782bf9-4051-4613-8852-33b0d85a0edd")
}

/// "Whenever a creature you control enters, you gain 1 life."
#[test]
fn ajani_s_welcome_gains_life_when_my_creature_enters() {
    let mut e = table(7101, &[index::GRIZZLY_BEARS], &[ajani_s_welcome()], &[]);
    assert_eq!(life(&e, P0), 20);
    cast_with_floating(&mut e, P0, index::GRIZZLY_BEARS);
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, P0, index::GRIZZLY_BEARS).is_some());
    assert_eq!(life(&e, P0), 21, "one life for the creature that entered");
    assert_eq!(life(&e, P1), 20);
}

/// A land entering is no creature.
#[test]
fn ajani_s_welcome_ignores_a_land_entering() {
    let mut e = table(7102, &[forest()], &[ajani_s_welcome()], &[]);
    play_land(&mut e, P0, forest());
    pass_until(&mut e, stack_is_empty);
    assert_eq!(life(&e, P0), 20);
}
