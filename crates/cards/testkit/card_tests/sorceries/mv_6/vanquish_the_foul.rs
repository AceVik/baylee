//! `cards/sorceries/mv_6/vanquish_the_foul.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "6990ac82-49d4-412d-9c3d-1e7fb5479313"
fn vanquish_the_foul() -> CardIndex {
    card_index("6990ac82-49d4-412d-9c3d-1e7fb5479313")
}

/// Vanquish the Foul: "Destroy target creature with power 4 or greater.
/// Scry 1."
///
/// Power four is the edge: the 4/5 Elemental is on the menu and the 2/2 Ogre is
/// not; the scry looks at one card.
#[test]
fn vanquish_the_foul_destroys_a_big_creature_and_scries_one() {
    let mut engine = table(
        980,
        &[vanquish_the_foul()],
        &[plains(); 6],
        &[],
        &[earth_elemental(), gray_ogre()],
    );
    let big = on_battlefield(&engine, them(), earth_elemental()).expect("their Elemental");
    let small = on_battlefield(&engine, them(), gray_ogre()).expect("their Ogre");
    cast_from_hand(&mut engine, me(), vanquish_the_foul());
    let menu = aim_at(&mut engine, me(), big);
    assert!(menu.contains(&big), "{menu:?}");
    assert!(!menu.contains(&small), "power under four: {menu:?}");
    let cards = look_and_send(&mut engine, 0);
    assert_eq!(cards.len(), 1, "scry 1");
    settle(&mut engine);
    assert!(in_graveyard(&engine, them(), earth_elemental()).is_some());
    assert!(on_battlefield(&engine, them(), gray_ogre()).is_some());
}
