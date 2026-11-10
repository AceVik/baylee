//! `cards/sorceries/mv_6/sip_of_hemlock.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "861096be-dbb3-429e-820f-d82f6ed2aa24"
fn sip_of_hemlock() -> CardIndex {
    card_index("861096be-dbb3-429e-820f-d82f6ed2aa24")
}

/// Sip of Hemlock: "Destroy target creature. Its controller loses 2 life."
#[test]
fn sip_of_hemlock_costs_the_creatures_controller_two_life() {
    let mut engine = table(973, &[sip_of_hemlock()], &[swamp(); 6], &[], &[gray_ogre()]);
    let ogre = on_battlefield(&engine, them(), gray_ogre()).expect("their Ogre");
    cast_at(&mut engine, sip_of_hemlock(), ogre);
    assert!(in_graveyard(&engine, them(), gray_ogre()).is_some());
    assert_eq!(life_of(&engine, them()), 18);
    assert_eq!(life_of(&engine, me()), 20, "the caster is untouched");
}

/// "Its controller" is the creature's: aimed at my own, I lose two.
#[test]
fn sip_of_hemlock_aimed_at_my_own_creature_costs_me_two() {
    let mut engine = table(974, &[sip_of_hemlock()], &[swamp(); 6], &[gray_ogre()], &[]);
    let ogre = on_battlefield(&engine, me(), gray_ogre()).expect("my Ogre");
    cast_at(&mut engine, sip_of_hemlock(), ogre);
    assert_eq!(life_of(&engine, me()), 18);
    assert_eq!(life_of(&engine, them()), 20);
}
