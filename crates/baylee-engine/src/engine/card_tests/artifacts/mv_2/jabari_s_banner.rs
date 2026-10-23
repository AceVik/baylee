//! `cards/artifacts/mv_2/jabari_s_banner.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::mana::ManaColor;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn jabari_s_banner() -> CardIndex {
    card_index("79560d80-1745-4773-a5c4-4c8e98475135")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn wall_of_ice() -> CardIndex {
    card_index("1945e165-7636-4727-89ba-3811251ef175")
}

fn shadow_rider() -> CardIndex {
    card_index("a518b31a-7e9a-460d-839f-39cb1de4f570")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

/// Pays `{1}` and taps the Banner for `target`; returns the target menu.
#[track_caller]
fn flank(e: &mut Engine<RegistryLookup>, target: ObjectId) -> Vec<ObjectId> {
    lk::float(e, P0, &[(ManaColor::Colorless, 1)]);
    activate(e, P0, jabari_s_banner(), 0);
    let menu = aim_at(e, P0, target);
    lk::settle(e);
    menu
}

/// Jabari's Banner (79560d80-1745-4773-a5c4-4c8e98475135) — "{1}, {T}: Target
/// creature gains flanking until end of turn." The cost is paid: it is not
/// offered without the {1}, and it taps the Banner. The target is any
/// creature, the opponent's too; a blocking Wall of Ice is shrunk by exactly
/// one -1/-1 (the engine raises the flanking trigger from the keyword).
#[test]
fn jabari_s_banner_gives_flanking_and_costs_one_and_a_tap() {
    let mut e = lk::game(
        SEED,
        &[jabari_s_banner(), grizzly_bears()],
        &[wall_of_ice()],
        &[],
        &[],
    );
    let banner = lk::obj(&e, P0, jabari_s_banner());
    let bears = lk::obj(&e, P0, grizzly_bears());
    let wall = lk::obj(&e, P1, wall_of_ice());

    assert!(
        !lk::offered(&e, jabari_s_banner(), P0, 0),
        "no {{1}}, no activation"
    );
    assert!(!keywords(&e, bears).contains(KeywordSet::FLANKING));

    let menu = flank(&mut e, bears);
    assert!(
        menu.contains(&wall) && menu.contains(&bears),
        "any creature: {menu:?}"
    );
    assert!(lk::tapped(&e, banner), "{{T}} is part of the cost");
    assert!(keywords(&e, bears).contains(KeywordSet::FLANKING));
    assert!(!keywords(&e, wall).contains(KeywordSet::FLANKING));

    lk::attack(&mut e, &[bears]);
    lk::block(&mut e, &[(wall, bears)]);
    lk::settle(&mut e);
    assert_eq!(pt(&e, wall), (-1, 6), "-1/-1, once");
}

/// Jabari's Banner — "until end of turn": the next turn the Bears attack
/// without flanking and the Wall of Ice that blocks them keeps its 0/7.
#[test]
fn jabari_s_banner_flanking_ends_with_the_turn() {
    let mut e = lk::game(
        SEED,
        &[jabari_s_banner(), grizzly_bears()],
        &[wall_of_ice()],
        &[],
        &[],
    );
    let bears = lk::obj(&e, P0, grizzly_bears());
    let wall = lk::obj(&e, P1, wall_of_ice());
    flank(&mut e, bears);

    reach_their_main_phase(&mut e, P1);
    assert!(!keywords(&e, bears).contains(KeywordSet::FLANKING));
    reach_their_main_phase(&mut e, P0);

    lk::attack(&mut e, &[bears]);
    lk::block(&mut e, &[(wall, bears)]);
    lk::settle(&mut e);
    assert_eq!(pt(&e, wall), (0, 7), "no flanking on turn 3");
}

/// Jabari's Banner — "a creature without flanking": a blocking Shadow Rider
/// has flanking and is not shrunk; and the Lions that block the Hill Giant
/// (which the Banner did not touch) are not shrunk either.
#[test]
fn jabari_s_banner_shrinks_only_blockers_without_flanking_of_the_target() {
    let mut e = lk::game(
        SEED,
        &[jabari_s_banner(), grizzly_bears(), hill_giant()],
        &[shadow_rider(), savannah_lions()],
        &[],
        &[],
    );
    let bears = lk::obj(&e, P0, grizzly_bears());
    let giant = lk::obj(&e, P0, hill_giant());
    let rider = lk::obj(&e, P1, shadow_rider());
    let lions = lk::obj(&e, P1, savannah_lions());
    flank(&mut e, bears);

    lk::attack(&mut e, &[bears, giant]);
    lk::block(&mut e, &[(rider, bears), (lions, giant)]);
    lk::settle(&mut e);
    assert_eq!(
        pt(&e, rider),
        (3, 3),
        "a blocker with flanking is not shrunk"
    );
    assert_eq!(pt(&e, lions), (2, 1), "the Giant has no flanking");
}
