//! `cards/creatures/mv_1/goblin_vandal.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn goblin_vandal() -> CardIndex {
    card_index("50bebbb9-01b7-4eb6-8efd-307c9bcd2517")
}

fn jayemdae_tome() -> CardIndex {
    card_index("39ee576a-0803-4063-9c84-f2b537e4d44c")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// P0's Vandal and Mountain, an artifact on each side; the Vandal has attacked
/// and P1 declared no blocks, so the trigger is asking for its target.
fn unblocked_vandal() -> (Engine<RegistryLookup>, ObjectId, ObjectId, ObjectId) {
    let mut e = lk::game(
        SEED,
        &[goblin_vandal(), mountain(), jayemdae_tome()],
        &[jayemdae_tome()],
        &[],
        &[],
    );
    let vandal = lk::obj(&e, P0, goblin_vandal());
    let mine = lk::obj(&e, P0, jayemdae_tome());
    let theirs = lk::obj(&e, P1, jayemdae_tome());
    lk::attack(&mut e, &[vandal]);
    lk::block(&mut e, &[]);
    (e, vandal, mine, theirs)
}

/// Goblin Vandal (50bebbb9-01b7-4eb6-8efd-307c9bcd2517) — "Whenever this
/// creature attacks and isn't blocked, you may pay {R}. If you do, destroy
/// target artifact defending player controls and this creature assigns no
/// combat damage this turn." The target is the defender's artifact (the
/// caster's own is not offered); paying {R} with the Mountain destroys it,
/// and the unblocked 1/1 then deals no damage.
#[test]
fn goblin_vandal_paid_destroys_their_artifact_and_deals_no_combat_damage() {
    let (mut e, vandal, mine, theirs) = unblocked_vandal();
    let mountain_id = lk::obj(&e, P0, mountain());

    let menu = aim_at(&mut e, P0, theirs);
    assert!(menu.contains(&theirs), "the defender's artifact: {menu:?}");
    assert!(!menu.contains(&mine), "not the caster's own: {menu:?}");
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    e.apply(P0, PlayerAction::YesNo(true)).unwrap();
    e.apply(
        P0,
        PlayerAction::PayWindow {
            steps: vec![PlayerAction::ActivateManaAbility {
                source: mountain_id,
            }],
        },
    )
    .expect("the Mountain pays {R}");
    lk::through_combat(&mut e);

    assert!(in_graveyard(&e, P1, jayemdae_tome()).is_some(), "destroyed");
    assert!(
        on_battlefield(&e, P0, jayemdae_tome()).is_some(),
        "theirs only"
    );
    assert_eq!(lk::life(&e, P1), 20, "and no combat damage went through");
    assert!(on_battlefield(&e, P0, goblin_vandal()).is_some());
    assert_ne!(lk::zone_of(&e, vandal), crate::zone::Zone::Graveyard);
}

/// Goblin Vandal — "you may pay {R}": declined, the artifact stays and the
/// Vandal deals its 1 damage.
#[test]
fn goblin_vandal_declined_destroys_nothing_and_deals_its_damage() {
    let (mut e, _vandal, _mine, theirs) = unblocked_vandal();
    let mountain_id = lk::obj(&e, P0, mountain());

    aim_at(&mut e, P0, theirs);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    e.apply(P0, PlayerAction::YesNo(false)).unwrap();
    lk::through_combat(&mut e);

    assert!(
        on_battlefield(&e, P1, jayemdae_tome()).is_some(),
        "not destroyed"
    );
    assert!(!lk::tapped(&e, mountain_id), "nothing was paid");
    assert_eq!(lk::life(&e, P1), 19, "the 1/1 hit");
}

/// Goblin Vandal — "and isn't blocked": blocked by a Grizzly Bears it asks
/// nothing and destroys nothing.
#[test]
fn goblin_vandal_blocked_does_not_trigger() {
    let mut e = lk::game(
        SEED,
        &[goblin_vandal(), mountain()],
        &[jayemdae_tome(), grizzly_bears()],
        &[],
        &[],
    );
    let vandal = lk::obj(&e, P0, goblin_vandal());
    let bears = lk::obj(&e, P1, grizzly_bears());

    lk::attack(&mut e, &[vandal]);
    lk::block(&mut e, &[(bears, vandal)]);
    lk::through_combat(&mut e);

    assert!(
        on_battlefield(&e, P1, jayemdae_tome()).is_some(),
        "no trigger, no destruction"
    );
}

/// Goblin Vandal — "target artifact defending player controls": with no
/// artifact over there the unblocked Vandal is just a 1/1 and asks nothing.
#[test]
fn goblin_vandal_without_a_target_artifact_just_hits() {
    let mut e = lk::game(
        SEED,
        &[goblin_vandal(), mountain()],
        &[grizzly_bears()],
        &[],
        &[],
    );
    let vandal = lk::obj(&e, P0, goblin_vandal());

    lk::attack(&mut e, &[vandal]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);

    assert_eq!(lk::life(&e, P1), 19);
}
