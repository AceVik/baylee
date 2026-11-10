//! `cards/instants/mv_5/final_death.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "1be3eea4-eaff-469c-9689-abadac1e848c"
fn final_death() -> CardIndex {
    card_index("1be3eea4-eaff-469c-9689-abadac1e848c")
}

/// Final Death: "Exile target creature."
///
/// Either seat's creature is a legal target, and the one named ends in exile,
/// not the graveyard.
#[test]
fn final_death_exiles_a_creature_of_either_seat() {
    let mut engine = table(
        906,
        &[final_death()],
        &[swamp(); 5],
        &[gray_ogre()],
        &[llanowar_elves()],
    );
    let mine = on_battlefield(&engine, me(), gray_ogre()).expect("my Ogre");
    let theirs = on_battlefield(&engine, them(), llanowar_elves()).expect("their Elf");
    let menu = cast_at(&mut engine, final_death(), theirs);
    assert!(menu.contains(&mine) && menu.contains(&theirs), "{menu:?}");
    assert!(exiled(&engine, them(), llanowar_elves()), "exiled");
    assert!(in_graveyard(&engine, them(), llanowar_elves()).is_none());
    assert!(
        on_battlefield(&engine, me(), gray_ogre()).is_some(),
        "the creature not named stays"
    );
}
