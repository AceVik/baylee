//! `cards/creatures/mv_4/goblin_grenadiers.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn goblin_grenadiers() -> CardIndex {
    card_index("be83a518-cfab-46b1-9fe9-7e3d54311c37")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// P0's Grenadiers attack P1's Bears and Hill Giant unblocked; the trigger
/// asks for its creature target. (The land half of the card is not
/// implemented, so there is no land target to give.)
fn unblocked_grenadiers() -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let mut e = lk::game(
        SEED,
        &[goblin_grenadiers()],
        &[grizzly_bears(), hill_giant()],
        &[],
        &[],
    );
    let grenadiers = lk::obj(&e, P0, goblin_grenadiers());
    let bears = lk::obj(&e, P1, grizzly_bears());
    let giant = lk::obj(&e, P1, hill_giant());
    lk::attack(&mut e, &[grenadiers]);
    lk::block(&mut e, &[]);
    (e, bears, giant)
}

/// Goblin Grenadiers (be83a518-cfab-46b1-9fe9-7e3d54311c37), the creature half
/// of "you may sacrifice it. If you do, destroy target creature and target
/// land": sacrificed, it destroys the targeted Hill Giant and not the Bears
/// beside it, and deals no combat damage (it is gone).
#[test]
fn goblin_grenadiers_sacrificed_destroys_only_the_targeted_creature() {
    let (mut e, bears, giant) = unblocked_grenadiers();

    let menu = aim_at(&mut e, P0, giant);
    assert!(
        menu.contains(&bears) && menu.contains(&giant),
        "any creature: {menu:?}"
    );
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    e.apply(P0, PlayerAction::YesNo(true)).unwrap();
    lk::through_combat(&mut e);

    assert!(
        in_graveyard(&e, P0, goblin_grenadiers()).is_some(),
        "sacrificed"
    );
    assert!(
        in_graveyard(&e, P1, hill_giant()).is_some(),
        "the target died"
    );
    assert!(
        on_battlefield(&e, P1, grizzly_bears()).is_some(),
        "the other creature lives"
    );
    assert_eq!(lk::life(&e, P1), 20, "it was sacrificed before damage");
}

/// Goblin Grenadiers — "you may sacrifice it": declined, nothing is
/// destroyed and the Goblin deals its 2 combat damage.
#[test]
fn goblin_grenadiers_declined_destroys_nothing_and_hits() {
    let (mut e, _bears, giant) = unblocked_grenadiers();

    aim_at(&mut e, P0, giant);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    e.apply(P0, PlayerAction::YesNo(false)).unwrap();
    lk::through_combat(&mut e);

    assert!(
        on_battlefield(&e, P0, goblin_grenadiers()).is_some(),
        "not sacrificed"
    );
    assert!(
        on_battlefield(&e, P1, hill_giant()).is_some(),
        "nothing destroyed"
    );
    assert_eq!(lk::life(&e, P1), 18);
}

/// Goblin Grenadiers — "and isn't blocked": blocked by the Bears it asks
/// nothing, destroys nothing and fights as a 2/2.
#[test]
fn goblin_grenadiers_blocked_does_not_trigger() {
    let mut e = lk::game(
        SEED,
        &[goblin_grenadiers()],
        &[grizzly_bears(), hill_giant()],
        &[],
        &[],
    );
    let grenadiers = lk::obj(&e, P0, goblin_grenadiers());
    let bears = lk::obj(&e, P1, grizzly_bears());

    lk::attack(&mut e, &[grenadiers]);
    lk::block(&mut e, &[(bears, grenadiers)]);
    lk::through_combat(&mut e);

    assert!(
        on_battlefield(&e, P1, hill_giant()).is_some(),
        "no trigger, no destruction"
    );
}
