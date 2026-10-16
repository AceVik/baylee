//! `cards/sorceries/mv_6/poison_arrow.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "6d920c99-8a85-4b20-97c2-7aa0c79ae70a"
fn poison_arrow() -> CardIndex {
    card_index("6d920c99-8a85-4b20-97c2-7aa0c79ae70a")
}

/// Poison Arrow: "Destroy target nonblack creature. You gain 3 life."
#[test]
fn poison_arrow_destroys_a_nonblack_creature_and_gains_three() {
    let mut engine = table(
        969,
        &[poison_arrow()],
        &[swamp(); 6],
        &[],
        &[gray_ogre(), lotleth_troll()],
    );
    let ogre = on_battlefield(&engine, them(), gray_ogre()).expect("their Ogre");
    let black = on_battlefield(&engine, them(), lotleth_troll()).expect("their Troll");
    let menu = cast_at(&mut engine, poison_arrow(), ogre);
    assert!(menu.contains(&ogre), "{menu:?}");
    assert!(!menu.contains(&black), "a black creature is no target: {menu:?}");
    assert!(in_graveyard(&engine, them(), gray_ogre()).is_some());
    assert_eq!(life_of(&engine, me()), 23);
}
