//! `cards/sorceries/mv_6/certain_death.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "1d4b8a31-6500-4e3d-bcaf-f6f184d7743e"
fn certain_death() -> CardIndex {
    card_index("1d4b8a31-6500-4e3d-bcaf-f6f184d7743e")
}

/// Certain Death: "Destroy target creature. Its controller loses 2 life and you
/// gain 2 life."
#[test]
fn certain_death_drains_two_from_the_creatures_controller() {
    let mut engine = table(948, &[certain_death()], &[swamp(); 6], &[], &[gray_ogre()]);
    let ogre = on_battlefield(&engine, them(), gray_ogre()).expect("their Ogre");
    cast_at(&mut engine, certain_death(), ogre);
    assert!(in_graveyard(&engine, them(), gray_ogre()).is_some());
    assert_eq!(life_of(&engine, them()), 18, "their controller loses two");
    assert_eq!(life_of(&engine, me()), 22, "and I gain two");
}

/// "Its controller" is read off the creature, not off the table: aimed at the
/// caster's own creature, the caster is the one who loses two (and gains two),
/// and the opponent's life does not move.
#[test]
fn certain_death_aimed_at_my_own_creature_leaves_the_opponent_alone() {
    let mut engine = table(949, &[certain_death()], &[swamp(); 6], &[gray_ogre()], &[]);
    let ogre = on_battlefield(&engine, me(), gray_ogre()).expect("my Ogre");
    cast_at(&mut engine, certain_death(), ogre);
    assert!(in_graveyard(&engine, me(), gray_ogre()).is_some());
    assert_eq!(life_of(&engine, them()), 20, "the opponent controls nothing here");
    assert_eq!(life_of(&engine, me()), 20, "minus two and plus two");
}
