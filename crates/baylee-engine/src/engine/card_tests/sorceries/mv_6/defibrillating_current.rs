//! `cards/sorceries/mv_6/defibrillating_current.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "5a96b93b-bae6-48fb-87f5-05f3ffcf7ba9"
fn defibrillating_current() -> CardIndex {
    card_index("5a96b93b-bae6-48fb-87f5-05f3ffcf7ba9")
}

/// Defibrillating Current: "deals 4 damage to target creature or planeswalker
/// and you gain 2 life." Printed `{2/R}{2/W}{2/B}`: three lands, one of each
/// colour, pay it.
///
/// Four damage kills the Ogre and not an Elemental with toughness five, and
/// only creatures are offered, no player.
#[test]
fn defibrillating_current_deals_four_to_a_creature_and_gains_two() {
    let mut engine = table(
        953,
        &[defibrillating_current()],
        &[mountain(), plains(), swamp()],
        &[],
        &[gray_ogre(), earth_elemental()],
    );
    let ogre = on_battlefield(&engine, them(), gray_ogre()).expect("their Ogre");
    let big = on_battlefield(&engine, them(), earth_elemental()).expect("their Elemental");
    cast_from_hand(&mut engine, me(), defibrillating_current());
    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        panic!("a target choice, got {:?}", engine.pending())
    };
    assert!(player_options.is_empty(), "no players: {player_options:?}");
    let menu = aim_at(&mut engine, me(), ogre);
    assert!(menu.contains(&ogre) && menu.contains(&big), "{menu:?}");
    settle(&mut engine);
    assert!(in_graveyard(&engine, them(), gray_ogre()).is_some(), "four damage on a 2/2");
    assert!(on_battlefield(&engine, them(), earth_elemental()).is_some(), "not named");
    assert_eq!(life_of(&engine, me()), 22);
    assert_eq!(life_of(&engine, them()), 20);
}

/// The planeswalker half of the filter: Jace is cast first (a seeded walker
/// has no loyalty), then takes four damage, which removes his three counters.
#[test]
fn defibrillating_current_kills_a_planeswalker() {
    let lands = [
        vec![island(); 2],
        vec![forest(); 2],
        vec![mountain(), plains(), swamp()],
    ]
    .concat();
    let mut engine = table(
        954,
        &[jace_the_mind_sculptor(), defibrillating_current()],
        &lands,
        &[],
        &[],
    );
    tap_only(&mut engine, island());
    tap_only(&mut engine, forest());
    cast_with_floating(&mut engine, me(), jace_the_mind_sculptor());
    settle(&mut engine);
    let jace = on_battlefield(&engine, me(), jace_the_mind_sculptor()).expect("Jace resolved");

    tap_all_mana(&mut engine, me());
    cast_with_floating(&mut engine, me(), defibrillating_current());
    let menu = aim_at(&mut engine, me(), jace);
    assert!(menu.contains(&jace), "a planeswalker is a legal target: {menu:?}");
    settle(&mut engine);
    assert!(
        in_graveyard(&engine, me(), jace_the_mind_sculptor()).is_some(),
        "four damage removes three loyalty"
    );
    assert_eq!(life_of(&engine, me()), 22);
}
