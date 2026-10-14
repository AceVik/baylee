//! `cards/enchantments/mv_1/molten_vortex.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn molten_vortex() -> CardIndex {
    card_index("da5ee877-143e-4f53-ad20-1bc527a8fff8")
}

/// "{R}, Discard a land card: This enchantment deals 2 damage to any target."
#[test]
fn molten_vortex_discards_a_land_for_two_damage() {
    let mut e = table(
        7221,
        &[forest(), index::GRIZZLY_BEARS],
        &[molten_vortex()],
        &[],
    );
    let land = in_hand(&e, P0, forest()).unwrap();
    activate(&mut e, P0, molten_vortex(), 0);
    drive(&mut e, &[], &[P1], &[land], true);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(life(&e, P1), 18);
    assert!(
        in_graveyard(&e, P0, forest()).is_some(),
        "the land was discarded"
    );
    assert!(
        in_hand(&e, P0, index::GRIZZLY_BEARS).is_some(),
        "the creature stayed"
    );
}

/// Only a land card may be discarded: with none in hand it is not offered.
#[test]
fn molten_vortex_is_not_offered_without_a_land_card() {
    let e = table(7222, &[index::GRIZZLY_BEARS], &[molten_vortex()], &[]);
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority expected")
    };
    assert!(legal.abilities.is_empty(), "{:?}", legal.abilities);
}
