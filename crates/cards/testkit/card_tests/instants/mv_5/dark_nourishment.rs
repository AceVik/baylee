//! `cards/instants/mv_5/dark_nourishment.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "95650a60-6f1d-4594-8b53-7fd406212207"
fn dark_nourishment() -> CardIndex {
    card_index("95650a60-6f1d-4594-8b53-7fd406212207")
}

/// Dark Nourishment: "deals 3 damage to any target. You gain 3 life."
///
/// "Any target" puts both players and creatures on the menu; a player is
/// named here and loses three while the caster gains three.
#[test]
fn dark_nourishment_hits_a_player_and_gains_three() {
    let mut engine = table(
        914,
        &[dark_nourishment()],
        &[swamp(); 5],
        &[],
        &[gray_ogre()],
    );
    let ogre = on_battlefield(&engine, them(), gray_ogre()).expect("their Ogre");
    let (objects, players) = cast_at_player(&mut engine, dark_nourishment(), them());
    assert!(
        objects.contains(&ogre),
        "a creature is a legal target: {objects:?}"
    );
    assert!(
        players.contains(&me()) && players.contains(&them()),
        "and so is either player: {players:?}"
    );
    assert_eq!(life_of(&engine, them()), 17, "three damage");
    assert_eq!(life_of(&engine, me()), 23, "three life");
    assert!(
        on_battlefield(&engine, them(), gray_ogre()).is_some(),
        "the Ogre was not named"
    );
}

/// The same card pointed at a creature: three damage kills a 2/2 and the life
/// is still gained.
#[test]
fn dark_nourishment_kills_a_creature_and_still_gains_three() {
    let mut engine = table(
        915,
        &[dark_nourishment()],
        &[swamp(); 5],
        &[],
        &[gray_ogre()],
    );
    let ogre = on_battlefield(&engine, them(), gray_ogre()).expect("their Ogre");
    cast_at(&mut engine, dark_nourishment(), ogre);
    assert!(
        in_graveyard(&engine, them(), gray_ogre()).is_some(),
        "3 damage on a 2/2"
    );
    assert_eq!(life_of(&engine, me()), 23);
    assert_eq!(life_of(&engine, them()), 20, "the player was not named");
}
