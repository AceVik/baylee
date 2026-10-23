//! `cards/creatures/mv_4/shadow_rider.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn shadow_rider() -> CardIndex {
    card_index("a518b31a-7e9a-460d-839f-39cb1de4f570")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn wall_of_ice() -> CardIndex {
    card_index("1945e165-7636-4727-89ba-3811251ef175")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

/// Shadow Rider (a518b31a-7e9a-460d-839f-39cb1de4f570) — "Flanking (Whenever
/// a creature without flanking blocks this creature, the blocking creature
/// gets -1/-1 until end of turn.)" The Bears that block it are a 1/1 for
/// exactly one -1/-1, not two.
#[test]
fn shadow_rider_shrinks_a_blocker_without_flanking_by_one() {
    let mut e = lk::game(SEED, &[shadow_rider()], &[grizzly_bears()], &[], &[]);
    let rider = lk::obj(&e, P0, shadow_rider());
    let bears = lk::obj(&e, P1, grizzly_bears());
    assert!(keywords(&e, rider).contains(KeywordSet::FLANKING));

    lk::attack(&mut e, &[rider]);
    assert_eq!(pt(&e, bears), (2, 2), "nothing before the block");
    lk::block(&mut e, &[(bears, rider)]);
    lk::settle(&mut e);
    assert_eq!(pt(&e, bears), (1, 1), "-1/-1 once");

    lk::to_step(&mut e, P0, Step::CombatEnd);
    assert!(
        in_graveyard(&e, P1, grizzly_bears()).is_some(),
        "the Rider's 3 damage killed the 1/1"
    );
}

/// Shadow Rider — "until end of turn": a Wall of Ice that survives the
/// combat is a 0/7 again in the opponent's turn.
#[test]
fn shadow_rider_shrink_ends_with_the_turn() {
    let mut e = lk::game(SEED, &[shadow_rider()], &[wall_of_ice()], &[], &[]);
    let rider = lk::obj(&e, P0, shadow_rider());
    let wall = lk::obj(&e, P1, wall_of_ice());

    lk::attack(&mut e, &[rider]);
    lk::block(&mut e, &[(wall, rider)]);
    lk::settle(&mut e);
    assert_eq!(pt(&e, wall), (-1, 6));
    lk::to_step(&mut e, P0, Step::CombatEnd);
    assert!(
        on_battlefield(&e, P1, wall_of_ice()).is_some(),
        "3 damage on a 6 toughness"
    );

    reach_their_main_phase(&mut e, P1);
    assert_eq!(pt(&e, wall), (0, 7), "the -1/-1 ended with the turn");
}

/// Shadow Rider — "a creature *without flanking*": a blocking Shadow Rider
/// has flanking itself and keeps its 3/3.
#[test]
fn shadow_rider_does_not_shrink_a_blocker_with_flanking() {
    let mut e = lk::game(SEED, &[shadow_rider()], &[shadow_rider()], &[], &[]);
    let rider = lk::obj(&e, P0, shadow_rider());
    let theirs = lk::obj(&e, P1, shadow_rider());

    lk::attack(&mut e, &[rider]);
    lk::block(&mut e, &[(theirs, rider)]);
    lk::settle(&mut e);
    assert_eq!(pt(&e, theirs), (3, 3));
}

/// Shadow Rider — "blocks *this* creature": the Lions that block the Hill
/// Giant beside the Rider are not shrunk, the Bears that block the Rider are.
#[test]
fn shadow_rider_shrinks_only_the_creature_that_blocks_it() {
    let mut e = lk::game(
        SEED,
        &[shadow_rider(), hill_giant()],
        &[grizzly_bears(), savannah_lions()],
        &[],
        &[],
    );
    let rider = lk::obj(&e, P0, shadow_rider());
    let giant = lk::obj(&e, P0, hill_giant());
    let bears = lk::obj(&e, P1, grizzly_bears());
    let lions = lk::obj(&e, P1, savannah_lions());

    lk::attack(&mut e, &[rider, giant]);
    lk::block(&mut e, &[(bears, rider), (lions, giant)]);
    lk::settle(&mut e);
    assert_eq!(pt(&e, bears), (1, 1), "blocks the Rider");
    assert_eq!(pt(&e, lions), (2, 1), "blocks a creature without flanking");
}
