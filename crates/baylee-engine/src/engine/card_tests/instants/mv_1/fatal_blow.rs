//! `cards/instants/mv_1/fatal_blow.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn fatal_blow() -> CardIndex {
    card_index("16774611-c004-4383-b4e1-4a43a9b1f4aa")
}

fn shock() -> CardIndex {
    card_index("a9d288b8-cdc1-4e55-a0c9-d6edfc95e65d")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn wall_of_bone() -> CardIndex {
    card_index("8ffe4986-9e09-4421-ad26-296a4c0df9e4")
}

/// Fatal Blow (16774611-c004-4383-b4e1-4a43a9b1f4aa) — "Destroy target
/// creature that was dealt damage this turn." After a Shock on the Hill Giant
/// the Giant is on the menu and the undamaged Bears are not; the Giant dies.
#[test]
fn fatal_blow_destroys_only_a_creature_dealt_damage_this_turn() {
    let mut e = lk::game(
        SEED,
        &[mountain(), swamp()],
        &[hill_giant(), grizzly_bears()],
        &[shock(), fatal_blow()],
        &[],
    );
    let giant = lk::obj(&e, P1, hill_giant());
    let bears = lk::obj(&e, P1, grizzly_bears());

    tap_all_mana(&mut e, P0);
    cast_with_floating(&mut e, P0, shock());
    aim_at(&mut e, P0, giant);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(lk::damage_on(&e, giant), 2, "the Shock marked the Giant");

    cast_with_floating(&mut e, P0, fatal_blow());
    let menu = aim_at(&mut e, P0, giant);
    assert!(
        !menu.contains(&bears),
        "the Bears were not dealt damage: {menu:?}"
    );
    pass_until(&mut e, stack_is_empty);

    assert!(in_graveyard(&e, P1, hill_giant()).is_some());
    assert!(on_battlefield(&e, P1, grizzly_bears()).is_some());
}

/// Fatal Blow — "this turn": the Giant Shocked on turn 1 is healed by turn 3
/// and is no longer a creature that was dealt damage this turn, so with a
/// Fatal Blow and the mana in hand the spell has nothing to aim at.
#[test]
fn fatal_blow_needs_damage_from_this_turn_not_last_turn() {
    let mut e = lk::game(
        SEED,
        &[mountain(), swamp()],
        &[hill_giant()],
        &[shock(), fatal_blow()],
        &[],
    );
    let giant = lk::obj(&e, P1, hill_giant());
    tap_all_mana(&mut e, P0);
    cast_with_floating(&mut e, P0, shock());
    aim_at(&mut e, P0, giant);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(lk::damage_on(&e, giant), 2);

    pass_until(&mut e, |e| {
        e.state().turn.number == 3 && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(lk::damage_on(&e, giant), 0, "healed at cleanup");
    tap_all_mana(&mut e, P0);
    let spell = in_hand(&e, P0, fatal_blow()).expect("the Fatal Blow is still in hand");
    assert!(
        e.apply(P0, PlayerAction::CastSpell { card: spell })
            .is_err(),
        "no creature was dealt damage this turn"
    );
}

/// Fatal Blow — "It can't be regenerated." A Wall of Bone with a regeneration
/// shield standing is Shocked (it survives, 1/4) and then destroyed anyway.
#[test]
fn fatal_blow_kills_through_a_regeneration_shield() {
    let mut e = lk::game(
        SEED,
        &[wall_of_bone(), swamp(), swamp(), mountain()],
        &[],
        &[shock(), fatal_blow()],
        &[],
    );
    let wall = lk::obj(&e, P0, wall_of_bone());
    tap_all_mana(&mut e, P0);
    raise_a_shield(&mut e, P0, wall, 0);

    cast_with_floating(&mut e, P0, shock());
    aim_at(&mut e, P0, wall);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(lk::damage_on(&e, wall), 2);

    cast_with_floating(&mut e, P0, fatal_blow());
    aim_at(&mut e, P0, wall);
    pass_until(&mut e, stack_is_empty);
    assert!(
        in_graveyard(&e, P0, wall_of_bone()).is_some(),
        "the shield did not save it"
    );
}
