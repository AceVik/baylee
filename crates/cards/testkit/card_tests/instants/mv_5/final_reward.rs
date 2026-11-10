//! `cards/instants/mv_5/final_reward.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "e654242f-c7c5-4713-bbd0-26d41de8e2e7"
fn final_reward() -> CardIndex {
    card_index("e654242f-c7c5-4713-bbd0-26d41de8e2e7")
}

/// Final Reward: "Exile target creature."
///
/// Either seat's creature is a legal target, and the one named ends in exile,
/// not the graveyard.
#[test]
fn final_reward_exiles_a_creature_of_either_seat() {
    let mut engine = table(
        907,
        &[final_reward()],
        &[swamp(); 5],
        &[gray_ogre()],
        &[llanowar_elves()],
    );
    let mine = on_battlefield(&engine, me(), gray_ogre()).expect("my Ogre");
    let theirs = on_battlefield(&engine, them(), llanowar_elves()).expect("their Elf");
    let menu = cast_at(&mut engine, final_reward(), theirs);
    assert!(menu.contains(&mine) && menu.contains(&theirs), "{menu:?}");
    assert!(exiled(&engine, them(), llanowar_elves()), "exiled");
    assert!(in_graveyard(&engine, them(), llanowar_elves()).is_none());
    assert!(
        on_battlefield(&engine, me(), gray_ogre()).is_some(),
        "the creature not named stays"
    );
}
