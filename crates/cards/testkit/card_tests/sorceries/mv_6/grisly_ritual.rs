//! `cards/sorceries/mv_6/grisly_ritual.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "56c78349-e4fa-48ba-ae8e-d2312ae6edc6"
fn grisly_ritual() -> CardIndex {
    card_index("56c78349-e4fa-48ba-ae8e-d2312ae6edc6")
}

/// Grisly Ritual: "Destroy target creature or planeswalker. Create two Blood
/// tokens."
#[test]
fn grisly_ritual_destroys_a_creature_and_makes_two_blood() {
    let mut engine = table(959, &[grisly_ritual()], &[swamp(); 6], &[], &[gray_ogre()]);
    let ogre = on_battlefield(&engine, them(), gray_ogre()).expect("their Ogre");
    cast_at(&mut engine, grisly_ritual(), ogre);
    assert!(in_graveyard(&engine, them(), gray_ogre()).is_some());
    let blood = my_tokens(&engine);
    assert_eq!(blood.len(), 2, "two Blood tokens");
    for token in blood {
        assert!(
            types(&engine, token).contains(TypeSet::ARTIFACT),
            "Blood is an artifact"
        );
    }
}

/// The planeswalker half: a cast Jace is on the menu beside a creature, and is
/// the one destroyed.
#[test]
fn grisly_ritual_destroys_a_planeswalker() {
    let lands = [vec![island(); 2], vec![swamp(); 8]].concat();
    let mut engine = table(
        960,
        &[jace_the_mind_sculptor(), grisly_ritual()],
        &lands,
        &[],
        &[gray_ogre(), quiet_artifact()],
    );
    let ogre = on_battlefield(&engine, them(), gray_ogre()).expect("their Ogre");
    let rock = on_battlefield(&engine, them(), quiet_artifact()).expect("their artifact");
    tap_all_mana(&mut engine, me());
    cast_with_floating(&mut engine, me(), jace_the_mind_sculptor());
    settle(&mut engine);
    let jace = on_battlefield(&engine, me(), jace_the_mind_sculptor()).expect("Jace resolved");
    cast_with_floating(&mut engine, me(), grisly_ritual());
    let menu = aim_at(&mut engine, me(), jace);
    assert!(menu.contains(&jace) && menu.contains(&ogre), "{menu:?}");
    assert!(!menu.contains(&rock), "{menu:?}");
    settle(&mut engine);
    assert!(in_graveyard(&engine, me(), jace_the_mind_sculptor()).is_some());
    assert_eq!(my_tokens(&engine).len(), 2, "two Blood");
}
