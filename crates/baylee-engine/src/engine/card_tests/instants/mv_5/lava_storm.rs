//! `cards/instants/mv_5/lava_storm.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::turn::Step;
use baylee_core::mana::ManaColor;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn lava_storm() -> CardIndex {
    card_index("78d33345-a147-402f-9a40-1e8260d7d44e")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn gray_ogre() -> CardIndex {
    card_index("83c8a3a6-2e1a-4e26-8847-6d066f42d906")
}

fn wall_of_ice() -> CardIndex {
    card_index("1945e165-7636-4727-89ba-3811251ef175")
}

/// Player 0 attacks with Grizzly Bears (2/2) and Hill Giant (3/3); player 1's
/// Gray Ogre (2/2) blocks the Bears and the Wall of Ice (0/7) blocks nothing.
/// Returns the engine in the declare-blockers step with Lava Storm cast and
/// the mode question pending.
fn storm_in_the_blocks_step() -> Engine<RegistryLookup> {
    let mut e = lk::game(
        SEED,
        &[grizzly_bears(), hill_giant()],
        &[gray_ogre(), wall_of_ice()],
        &[lava_storm()],
        &[],
    );
    let bears = lk::obj(&e, P0, grizzly_bears());
    let giant = lk::obj(&e, P0, hill_giant());
    let ogre = lk::obj(&e, P1, gray_ogre());

    lk::attack(&mut e, &[bears, giant]);
    lk::block(&mut e, &[(ogre, bears)]);
    lk::to_step(&mut e, P0, Step::DeclareBlockers);

    lk::float(&mut e, P0, &[(ManaColor::Red, 5)]);
    cast_with_floating(&mut e, P0, lava_storm());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    e
}

/// Lava Storm (78d33345-a147-402f-9a40-1e8260d7d44e) — "deals 2 damage to each
/// attacking creature": the 2/2 Bears dies, the 3/3 Giant survives with the
/// damage marked, and neither creature of the defender (the Ogre that blocked,
/// the Wall that did not) is touched.
#[test]
fn lava_storm_first_mode_hits_each_attacking_creature_only() {
    let mut e = storm_in_the_blocks_step();
    let giant = lk::obj(&e, P0, hill_giant());
    let ogre = lk::obj(&e, P1, gray_ogre());
    let wall = lk::obj(&e, P1, wall_of_ice());

    e.apply(P0, PlayerAction::ChooseMode(0)).unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(
        in_graveyard(&e, P0, grizzly_bears()).is_some(),
        "2 damage kills the Bears"
    );
    assert_eq!(
        lk::damage_on(&e, giant),
        2,
        "the Giant survives with 2 marked"
    );
    assert_eq!(lk::damage_on(&e, ogre), 0, "a blocker is not an attacker");
    assert_eq!(lk::damage_on(&e, wall), 0);
    assert!(on_battlefield(&e, P1, gray_ogre()).is_some());
}

/// Lava Storm — "or … 2 damage to each blocking creature": only the Ogre that
/// blocked dies; the Wall that blocked nothing and both attackers are
/// untouched.
#[test]
fn lava_storm_second_mode_hits_each_blocking_creature_only() {
    let mut e = storm_in_the_blocks_step();
    let bears = lk::obj(&e, P0, grizzly_bears());
    let giant = lk::obj(&e, P0, hill_giant());
    let wall = lk::obj(&e, P1, wall_of_ice());

    e.apply(P0, PlayerAction::ChooseMode(1)).unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(
        in_graveyard(&e, P1, gray_ogre()).is_some(),
        "2 damage kills the blocking Ogre"
    );
    assert_eq!(lk::damage_on(&e, wall), 0, "the Wall did not block");
    assert_eq!(lk::damage_on(&e, bears), 0, "an attacker is not a blocker");
    assert_eq!(lk::damage_on(&e, giant), 0);
}
