//! `cards/instants/mv_2/thunderbolt.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn thunderbolt() -> CardIndex {
    card_index("6ffae2fa-511a-45bf-93c4-5d53c85324f5")
}

fn air_elemental() -> CardIndex {
    card_index("7744bae4-a8b7-44a5-9b4c-0048ad4cc448")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// Thunderbolt (6ffae2fa-511a-45bf-93c4-5d53c85324f5), the first mode: "3
/// damage to target player or planeswalker". The player at 20 takes 3; the
/// creature menu is empty (neither the Hill Giant nor the flyer is a legal
/// target) and neither creature is hurt.
#[test]
fn thunderbolt_first_mode_deals_three_to_a_player_and_cannot_aim_at_a_creature() {
    let mut e = lk::game(
        SEED,
        &[],
        &[hill_giant(), air_elemental()],
        &[thunderbolt()],
        &[],
    );
    let giant = lk::obj(&e, P1, hill_giant());
    let flyer = lk::obj(&e, P1, air_elemental());

    lk::float(
        &mut e,
        P0,
        &[(ManaColor::Red, 1), (ManaColor::Colorless, 1)],
    );
    cast_with_floating(&mut e, P0, thunderbolt());
    e.apply(P0, PlayerAction::ChooseMode(0)).unwrap();
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = e.pending().clone()
    else {
        panic!("the mode asks for a target: {:?}", e.pending())
    };
    assert!(
        options.is_empty(),
        "no creature is a legal target: {options:?}"
    );
    assert!(player_options.contains(&P1));
    e.apply(
        P0,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![P1],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert_eq!(lk::life(&e, P1), 17, "3 damage");
    assert_eq!(lk::life(&e, P0), 20);
    assert_eq!(lk::damage_on(&e, giant), 0);
    assert_eq!(lk::damage_on(&e, flyer), 0);
}

/// Thunderbolt, the second mode: "4 damage to target creature with flying".
/// The menu holds the flyers (the opponent's and the caster's own) and not the
/// ground Hill Giant; the opponent's 4/4 Air Elemental dies, the caster's own
/// flyer, the Giant and the player at 20 are untouched.
#[test]
fn thunderbolt_second_mode_hits_only_the_chosen_flying_creature() {
    let mut e = lk::game(
        SEED,
        &[air_elemental()],
        &[hill_giant(), air_elemental()],
        &[thunderbolt()],
        &[],
    );
    let mine = lk::obj(&e, P0, air_elemental());
    let giant = lk::obj(&e, P1, hill_giant());
    let theirs = lk::obj(&e, P1, air_elemental());

    lk::float(
        &mut e,
        P0,
        &[(ManaColor::Red, 1), (ManaColor::Colorless, 1)],
    );
    cast_with_floating(&mut e, P0, thunderbolt());
    e.apply(P0, PlayerAction::ChooseMode(1)).unwrap();
    let menu = aim_at(&mut e, P0, theirs);
    assert!(
        menu.contains(&theirs) && menu.contains(&mine),
        "any flyer: {menu:?}"
    );
    assert!(
        !menu.contains(&giant),
        "a ground creature is not a target: {menu:?}"
    );
    pass_until(&mut e, stack_is_empty);

    assert!(
        in_graveyard(&e, P1, air_elemental()).is_some(),
        "4 damage kills a 4/4"
    );
    assert!(
        on_battlefield(&e, P0, air_elemental()).is_some(),
        "the caster's own flyer lives"
    );
    assert_eq!(lk::damage_on(&e, mine), 0);
    assert_eq!(lk::damage_on(&e, giant), 0);
    assert_eq!(lk::life(&e, P1), 20, "the second mode hurts no player");
}
