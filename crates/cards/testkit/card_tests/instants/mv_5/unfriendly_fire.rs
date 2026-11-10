//! `cards/instants/mv_5/unfriendly_fire.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "ebb5b792-3906-4cb5-b8f9-063f94c9d135"
fn unfriendly_fire() -> CardIndex {
    card_index("ebb5b792-3906-4cb5-b8f9-063f94c9d135")
}

/// Unfriendly Fire: "deals 4 damage to any target."
#[test]
fn unfriendly_fire_hits_a_player_for_four() {
    let mut engine = table(
        937,
        &[unfriendly_fire()],
        &[mountain(); 5],
        &[],
        &[gray_ogre()],
    );
    let ogre = on_battlefield(&engine, them(), gray_ogre()).expect("their Ogre");
    let (objects, players) = cast_at_player(&mut engine, unfriendly_fire(), them());
    assert!(
        objects.contains(&ogre),
        "a creature is a target: {objects:?}"
    );
    assert!(
        players.contains(&me()) && players.contains(&them()),
        "{players:?}"
    );
    assert_eq!(life_of(&engine, them()), 16, "four damage");
    assert_eq!(life_of(&engine, me()), 20);
}

/// Four damage is lethal to the 2/2 and is not to a toughness-five body.
#[test]
fn unfriendly_fire_kills_a_creature_it_can_kill() {
    let mut engine = table(
        938,
        &[unfriendly_fire()],
        &[mountain(); 5],
        &[],
        &[gray_ogre()],
    );
    let ogre = on_battlefield(&engine, them(), gray_ogre()).expect("their Ogre");
    cast_at(&mut engine, unfriendly_fire(), ogre);
    assert!(in_graveyard(&engine, them(), gray_ogre()).is_some());
    assert_eq!(life_of(&engine, them()), 20);
}

#[test]
fn unfriendly_fire_leaves_a_toughness_five_creature_standing() {
    let mut engine = table(
        939,
        &[unfriendly_fire()],
        &[mountain(); 5],
        &[],
        &[earth_elemental()],
    );
    let big = on_battlefield(&engine, them(), earth_elemental()).expect("their Elemental");
    cast_at(&mut engine, unfriendly_fire(), big);
    assert!(
        on_battlefield(&engine, them(), earth_elemental()).is_some(),
        "four damage does not kill toughness five"
    );
}
