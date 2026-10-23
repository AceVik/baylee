//! `cards/creatures/mv_4/cinder_giant.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn cinder_giant() -> CardIndex {
    card_index("3a8306ae-3162-4105-95f4-dacd10c7c051")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

/// Cinder Giant (3a8306ae-3162-4105-95f4-dacd10c7c051) — "At the beginning of
/// your upkeep, this creature deals 2 damage to each other creature you
/// control." P0 controls a Bears (dies) and a Hill Giant (survives, damaged);
/// P1's Lions are not "you control" and are untouched. Nothing happens at
/// P1's upkeep, and the Giant does not hurt itself.
#[test]
fn cinder_giant_hurts_each_other_creature_you_control_at_your_upkeep_only() {
    let mut e = lk::game(
        SEED,
        &[grizzly_bears(), hill_giant()],
        &[savannah_lions()],
        &[cinder_giant()],
        &[],
    );
    lk::float(
        &mut e,
        P0,
        &[(ManaColor::Red, 1), (ManaColor::Colorless, 3)],
    );
    cast_with_floating(&mut e, P0, cinder_giant());
    pass_until(&mut e, stack_is_empty);
    let giant = lk::obj(&e, P0, cinder_giant());
    let hill = lk::obj(&e, P0, hill_giant());
    let lions = lk::obj(&e, P1, savannah_lions());

    // P1's upkeep comes first, and it is not "your upkeep".
    reach_their_main_phase(&mut e, P1);
    assert!(in_graveyard(&e, P0, grizzly_bears()).is_none());
    assert_eq!(
        lk::damage_on(&e, hill),
        0,
        "nothing at the opponent's upkeep"
    );

    // P0's turn-3 upkeep.
    pass_until(&mut e, |e| {
        e.state().turn.number == 3 && e.state().turn.phase == Phase::FirstMain
    });
    assert!(
        in_graveyard(&e, P0, grizzly_bears()).is_some(),
        "two damage kills the Bears"
    );
    assert_eq!(lk::damage_on(&e, hill), 2, "and marks the Hill Giant");
    assert_eq!(lk::damage_on(&e, giant), 0, "the Giant says \"other\"");
    assert_eq!(
        lk::damage_on(&e, lions),
        0,
        "the opponent's Lions are not creatures you control"
    );
    assert!(on_battlefield(&e, P1, savannah_lions()).is_some());
}

/// Cinder Giant's upkeep trigger has no intervening if: removing the Giant
/// in response does not stop its damage to the other creatures you control.
#[test]
fn cinder_giant_upkeep_damage_resolves_after_the_source_is_removed() {
    let mut e = lk::game(
        SEED,
        &[grizzly_bears(), hill_giant()],
        &[savannah_lions()],
        &[
            cinder_giant(),
            baylee_core::generated::index::LIGHTNING_BOLT,
        ],
        &[],
    );
    lk::float(
        &mut e,
        P0,
        &[(ManaColor::Red, 1), (ManaColor::Colorless, 3)],
    );
    cast_with_floating(&mut e, P0, cinder_giant());
    lk::settle(&mut e);
    let giant = lk::obj(&e, P0, cinder_giant());
    let hill = lk::obj(&e, P0, hill_giant());
    let lions = lk::obj(&e, P1, savannah_lions());
    pass_until(&mut e, |e| {
        e.state().turn.number == 3
            && e.state().turn.step == crate::turn::Step::Upkeep
            && !stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    lk::float(&mut e, P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut e, P0, baylee_core::generated::index::LIGHTNING_BOLT);
    aim_at(&mut e, P0, giant);
    pass_until(&mut e, |e| in_graveyard(e, P0, cinder_giant()).is_some());
    assert!(
        !stack_is_empty(&e),
        "the upkeep trigger remains on the stack"
    );
    assert_eq!(lk::damage_on(&e, hill), 0, "it has not resolved yet");
    lk::settle(&mut e);
    assert!(
        in_graveyard(&e, P0, grizzly_bears()).is_some(),
        "two upkeep damage"
    );
    assert_eq!(
        lk::damage_on(&e, hill),
        2,
        "other own creature still takes two"
    );
    assert_eq!(lk::damage_on(&e, lions), 0, "opposing creature takes none");
    assert!(on_battlefield(&e, P1, savannah_lions()).is_some());
}
