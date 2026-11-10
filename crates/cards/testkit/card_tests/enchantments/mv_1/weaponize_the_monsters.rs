//! `cards/enchantments/mv_1/weaponize_the_monsters.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn weaponize_the_monsters() -> CardIndex {
    card_index("fa2154fb-6756-4cf8-85a6-68264fb7a0b2")
}

/// "{2}, Sacrifice a creature: This enchantment deals 2 damage to any target."
#[test]
fn weaponize_the_monsters_sacrifices_a_creature_for_two_damage() {
    let mut e = table(
        7251,
        &[],
        &[weaponize_the_monsters(), index::GRIZZLY_BEARS],
        &[],
    );
    let bears = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    activate(&mut e, P0, weaponize_the_monsters(), 0);
    drive(&mut e, &[], &[P1], &[bears], true);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(life(&e, P1), 18);
    assert!(in_graveyard(&e, P0, index::GRIZZLY_BEARS).is_some());
}
