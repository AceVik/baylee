//! `cards/creatures/mv_3/master_of_arms.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::mana::ManaColor;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn master_of_arms() -> CardIndex {
    card_index("37e90cab-d904-4387-9d18-62f125c38c46")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

/// P0's Master of Arms (no lands, so only floating mana can pay for it) has
/// attacked a board of Grizzly Bears and Savannah Lions; P1 blocks with the
/// Bears or not at all. Returns at P0's priority in the declare blockers step.
fn attack_and_block(
    block_with_bears: bool,
) -> (Engine<RegistryLookup>, ObjectId, ObjectId, ObjectId) {
    let mut e = lk::game(
        SEED,
        &[master_of_arms()],
        &[grizzly_bears(), savannah_lions()],
        &[],
        &[],
    );
    let master = lk::obj(&e, P0, master_of_arms());
    let bears = lk::obj(&e, P1, grizzly_bears());
    let lions = lk::obj(&e, P1, savannah_lions());
    lk::attack(&mut e, &[master]);
    if block_with_bears {
        lk::block(&mut e, &[(bears, master)]);
    } else {
        lk::block(&mut e, &[]);
    }
    (e, master, bears, lions)
}

/// Master of Arms (37e90cab-d904-4387-9d18-62f125c38c46) — "{1}{W}: Tap target
/// creature blocking this creature." The ability needs its {1}{W} and a
/// blocker; the menu holds the Bears that block it and not the Lions that
/// do not; the Bears end up tapped and the Lions untapped.
#[test]
fn master_of_arms_taps_the_creature_blocking_it_and_only_that() {
    let (mut e, _master, bears, lions) = attack_and_block(true);

    assert!(
        !lk::offered(&e, master_of_arms(), P0, 0),
        "{{1}}{{W}} is not paid for"
    );
    lk::float(
        &mut e,
        P0,
        &[(ManaColor::White, 1), (ManaColor::Colorless, 1)],
    );
    assert!(
        lk::offered(&e, master_of_arms(), P0, 0),
        "blocked, and paid for"
    );

    activate(&mut e, P0, master_of_arms(), 0);
    let menu = aim_at(&mut e, P0, bears);
    assert!(menu.contains(&bears), "the blocker: {menu:?}");
    assert!(
        !menu.contains(&lions),
        "not the creature that is not blocking: {menu:?}"
    );
    lk::settle(&mut e);

    assert!(lk::tapped(&e, bears), "tapped");
    assert!(!lk::tapped(&e, lions), "the other creature is untouched");
    assert_eq!(lk::pool_total(&e, P0), 0, "{{1}}{{W}} was spent");
}

/// Master of Arms — "First strike": the 2/2 kills the blocking 2/2 Bears in
/// the first-strike step, before the Bears deal any damage.
#[test]
fn master_of_arms_has_first_strike() {
    let (mut e, master, _bears, _lions) = attack_and_block(true);

    lk::through_combat(&mut e);

    assert!(
        in_graveyard(&e, P1, grizzly_bears()).is_some(),
        "the Bears died"
    );
    assert!(
        on_battlefield(&e, P0, master_of_arms()).is_some(),
        "the Master lives"
    );
    assert_eq!(
        lk::damage_on(&e, master),
        0,
        "dealt damage first, took none"
    );
}

/// Master of Arms — "creature blocking this creature": when nothing blocks
/// it there is no legal target, so the ability is not offered even with the
/// mana in the pool.
#[test]
fn master_of_arms_has_no_target_when_it_is_not_blocked() {
    let (mut e, _master, _bears, _lions) = attack_and_block(false);

    lk::float(
        &mut e,
        P0,
        &[(ManaColor::White, 1), (ManaColor::Colorless, 1)],
    );

    assert!(
        !lk::offered(&e, master_of_arms(), P0, 0),
        "no blocker to tap"
    );
}
