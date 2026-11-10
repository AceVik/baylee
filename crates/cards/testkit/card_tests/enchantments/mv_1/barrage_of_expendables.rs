//! `cards/enchantments/mv_1/barrage_of_expendables.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn barrage_of_expendables() -> CardIndex {
    card_index("da7600f6-5d4c-46bf-936a-9e0f0d508e19")
}

/// "{R}, Sacrifice a creature: This enchantment deals 1 damage to any target."
#[test]
fn barrage_of_expendables_sacrifices_a_creature_for_one_damage() {
    let mut e = table(
        7111,
        &[],
        &[barrage_of_expendables(), index::GRIZZLY_BEARS],
        &[],
    );
    let bears = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    activate(&mut e, P0, barrage_of_expendables(), 0);
    drive(&mut e, &[], &[P1], &[bears], true);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(life(&e, P1), 19);
    assert!(in_graveyard(&e, P0, index::GRIZZLY_BEARS).is_some());
    assert!(on_battlefield(&e, P0, barrage_of_expendables()).is_some());
}

/// The sacrifice is part of the cost: with no creature it is not offered.
#[test]
fn barrage_of_expendables_needs_a_creature_to_sacrifice() {
    let e = table(7112, &[], &[barrage_of_expendables()], &[]);
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority expected")
    };
    assert!(legal.abilities.is_empty(), "{:?}", legal.abilities);
}
