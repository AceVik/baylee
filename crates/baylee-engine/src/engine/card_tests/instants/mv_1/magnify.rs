//! `cards/instants/mv_1/magnify.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn magnify() -> CardIndex {
    card_index("421c661a-24d9-4797-b538-2a4243825628")
}

/// Magnify (421c661a-24d9-4797-b538-2a4243825628) — "All creatures get +1/+1
/// until end of turn."
///
/// Symmetrically pumps both players' creatures until cleanup, when the effect
/// wears off.
#[test]
fn magnify_pumps_all_creatures_on_both_sides_until_end_of_turn() {
    let mut e = lk::game(SEED, &[llanowar_elves()], &[gray_ogre()], &[magnify()], &[]);
    let elf = lk::obj(&e, lk::P0, llanowar_elves());
    let ogre = lk::obj(&e, lk::P1, gray_ogre());
    assert_eq!(pt(&e, elf), (1, 1));
    assert_eq!(pt(&e, ogre), (2, 2));

    lk::float(&mut e, lk::P0, &[(ManaColor::Green, 1)]);
    cast_with_floating(&mut e, lk::P0, magnify());
    lk::settle(&mut e);

    assert_eq!(pt(&e, elf), (2, 2), "caster's creature gets +1/+1");
    assert_eq!(
        pt(&e, ogre),
        (3, 3),
        "opponent's creature also gets +1/+1 ('All creatures')"
    );

    reach_their_main_phase(&mut e, lk::P1);
    assert_eq!(pt(&e, elf), (1, 1), "+1/+1 wears off at cleanup");
    assert_eq!(pt(&e, ogre), (2, 2), "+1/+1 wears off at cleanup");
}
