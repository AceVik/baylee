//! `cards/instants/mv_5/humble_the_brute.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "bea6606c-ea9a-4199-a8cd-b35d7c47fd7b"
fn humble_the_brute() -> CardIndex {
    card_index("bea6606c-ea9a-4199-a8cd-b35d7c47fd7b")
}

/// Humble the Brute: "Destroy target creature with power 4 or greater.
/// Investigate."
///
/// Power four is the edge: the 4/5 Earth Elemental is on the menu and the 2/2
/// Ogre and the 1/1 Elf are not. A Clue follows.
#[test]
fn humble_the_brute_destroys_a_big_creature_and_investigates() {
    let mut engine = table(
        921,
        &[humble_the_brute()],
        &[plains(); 5],
        &[],
        &[earth_elemental(), gray_ogre(), llanowar_elves()],
    );
    let big = on_battlefield(&engine, them(), earth_elemental()).expect("their Elemental");
    let small = on_battlefield(&engine, them(), gray_ogre()).expect("their Ogre");
    let elf = on_battlefield(&engine, them(), llanowar_elves()).expect("their Elf");
    assert!(my_tokens(&engine).is_empty());
    let menu = cast_at(&mut engine, humble_the_brute(), big);
    assert!(menu.contains(&big), "power four: {menu:?}");
    assert!(
        !menu.contains(&small) && !menu.contains(&elf),
        "power under four: {menu:?}"
    );
    assert!(in_graveyard(&engine, them(), earth_elemental()).is_some());
    let clues = my_tokens(&engine);
    assert_eq!(clues.len(), 1, "one Clue");
    assert!(types(&engine, clues[0]).contains(TypeSet::ARTIFACT));
}
