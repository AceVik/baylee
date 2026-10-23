//! `cards/instants/mv_6/spinning_darkness.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn spinning_darkness() -> CardIndex {
    card_index("69eeee95-cc69-41d1-833b-8895ef72df82")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn scathe_zombies() -> CardIndex {
    card_index("e0fefaf0-da20-4d58-8db7-019dba16c780")
}

fn wall_of_ice() -> CardIndex {
    card_index("1945e165-7636-4727-89ba-3811251ef175")
}

fn cast(e: &mut Engine<RegistryLookup>, target: ObjectId) -> Vec<ObjectId> {
    lk::float(e, P0, &[(ManaColor::Black, 6)]);
    cast_with_floating(e, P0, spinning_darkness());
    let menu = aim_at(e, P0, target);
    pass_until(e, stack_is_empty);
    menu
}

/// Spinning Darkness (69eeee95-cc69-41d1-833b-8895ef72df82) — "deals 3 damage
/// to target nonblack creature. You gain 3 life." The red Hill Giant dies and
/// the caster gains 3 (23); the black Scathe Zombies is not a legal target and
/// stays at 0 damage; the opponent's life does not change.
#[test]
fn spinning_darkness_hits_a_nonblack_creature_and_not_a_black_one() {
    let mut e = lk::game(
        SEED,
        &[],
        &[hill_giant(), scathe_zombies()],
        &[spinning_darkness()],
        &[],
    );
    let giant = lk::obj(&e, P1, hill_giant());
    let zombies = lk::obj(&e, P1, scathe_zombies());

    let menu = cast(&mut e, giant);

    assert!(
        menu.contains(&giant),
        "a nonblack creature is a target: {menu:?}"
    );
    assert!(
        !menu.contains(&zombies),
        "a black creature is not: {menu:?}"
    );
    assert!(
        in_graveyard(&e, P1, hill_giant()).is_some(),
        "3 damage kills a 3/3"
    );
    assert_eq!(lk::damage_on(&e, zombies), 0);
    assert_eq!(lk::life(&e, P0), 23, "the caster gains 3");
    assert_eq!(lk::life(&e, P1), 20);
}

/// Spinning Darkness — the life is gained whether or not the creature dies: the
/// blue 0/7 Wall of Ice survives the 3 damage (marked) and the caster still
/// gains 3.
#[test]
fn spinning_darkness_gains_life_even_if_the_creature_survives() {
    let mut e = lk::game(SEED, &[], &[wall_of_ice()], &[spinning_darkness()], &[]);
    let wall = lk::obj(&e, P1, wall_of_ice());

    cast(&mut e, wall);

    assert!(
        on_battlefield(&e, P1, wall_of_ice()).is_some(),
        "3 damage does not kill a 0/7"
    );
    assert_eq!(lk::damage_on(&e, wall), 3);
    assert_eq!(lk::life(&e, P0), 23);
}
