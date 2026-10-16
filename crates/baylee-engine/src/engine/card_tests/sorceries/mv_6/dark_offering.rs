//! `cards/sorceries/mv_6/dark_offering.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "102356a9-0de5-4034-b121-034da5deb4f2"
fn dark_offering() -> CardIndex {
    card_index("102356a9-0de5-4034-b121-034da5deb4f2")
}

/// Dark Offering: "Destroy target nonblack creature. You gain 3 life."
#[test]
fn dark_offering_destroys_a_nonblack_creature_and_gains_three() {
    let mut engine = table(
        952,
        &[dark_offering()],
        &[swamp(); 6],
        &[],
        &[gray_ogre(), lotleth_troll()],
    );
    let ogre = on_battlefield(&engine, them(), gray_ogre()).expect("their Ogre");
    let black = on_battlefield(&engine, them(), lotleth_troll()).expect("their Troll");
    let menu = cast_at(&mut engine, dark_offering(), ogre);
    assert!(menu.contains(&ogre), "{menu:?}");
    assert!(!menu.contains(&black), "a black creature is no target: {menu:?}");
    assert!(in_graveyard(&engine, them(), gray_ogre()).is_some());
    assert_eq!(life_of(&engine, me()), 23);
}
