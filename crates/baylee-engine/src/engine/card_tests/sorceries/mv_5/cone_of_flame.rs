//! `cards/sorceries/mv_5/cone_of_flame.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn cone_of_flame() -> CardIndex {
    card_index("62879f3d-ed6d-4004-8196-f3fd7f614695")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// Cone of Flame (62879f3d-ed6d-4004-8196-f3fd7f614695), the 3-damage part —
/// "deals … 3 damage to a third target": a player at 20 goes to 17, the
/// caster is untouched and a creature that is not the target is unmarked.
#[test]
fn cone_of_flame_deals_three_to_the_player_it_is_aimed_at_and_to_nothing_else() {
    let mut e = lk::game(
        SEED,
        &[mountain(), mountain(), mountain(), mountain(), mountain()],
        &[grizzly_bears()],
        &[cone_of_flame()],
        &[],
    );
    let bears = lk::obj(&e, P1, grizzly_bears());

    cast_from_hand(&mut e, P0, cone_of_flame());
    e.apply(
        P0,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![P1],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert_eq!(lk::life(&e, P1), 17);
    assert_eq!(lk::life(&e, P0), 20, "the caster is untouched");
    assert_eq!(lk::damage_on(&e, bears), 0, "the Bears were not the target");
}

/// Cone of Flame — "any target": a creature takes the 3 and the player beside
/// it does not (the Hill Giant, a 3/3, dies to exactly 3).
#[test]
fn cone_of_flame_deals_three_to_a_creature_target() {
    let mut e = lk::game(
        SEED,
        &[mountain(), mountain(), mountain(), mountain(), mountain()],
        &[hill_giant()],
        &[cone_of_flame()],
        &[],
    );
    let giant = lk::obj(&e, P1, hill_giant());

    cast_from_hand(&mut e, P0, cone_of_flame());
    let menu = aim_at(&mut e, P0, giant);
    assert!(
        menu.contains(&giant),
        "a creature is a legal target: {menu:?}"
    );
    pass_until(&mut e, stack_is_empty);

    assert!(
        in_graveyard(&e, P1, hill_giant()).is_some(),
        "3 damage on a 3/3"
    );
    assert_eq!(lk::life(&e, P1), 20, "and the player is not hit");
}
