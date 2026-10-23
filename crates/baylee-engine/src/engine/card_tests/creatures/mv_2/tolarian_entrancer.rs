//! `cards/creatures/mv_2/tolarian_entrancer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn tolarian_entrancer() -> CardIndex {
    card_index("50c8c7e2-1b45-44b2-a50b-3b514d4c4c42")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn controller(e: &Engine<RegistryLookup>, id: ObjectId) -> PlayerId {
    e.state().object(id).expect("the object exists").controller
}

/// Tolarian Entrancer (50c8c7e2-1b45-44b2-a50b-3b514d4c4c42) — "Whenever this
/// creature becomes blocked by a creature, gain control of that creature at end
/// of combat." The 1/1 attacks, the Bears blocks it and kills it; the Giant
/// that did not block is not taken. Through combat damage the Bears is still
/// the opponent's; at end of combat it becomes the attacker's (though the
/// Entrancer itself is dead) and stays so into the next turn.
#[test]
fn tolarian_entrancer_takes_the_blocker_at_end_of_combat_for_good() {
    let mut e = lk::game(
        SEED,
        &[tolarian_entrancer()],
        &[grizzly_bears(), hill_giant()],
        &[],
        &[],
    );
    let entrancer = lk::obj(&e, P0, tolarian_entrancer());
    let blocker = lk::obj(&e, P1, grizzly_bears());
    let bystander = lk::obj(&e, P1, hill_giant());

    lk::attack(&mut e, &[entrancer]);
    lk::block(&mut e, &[(blocker, entrancer)]);
    lk::to_step(&mut e, P0, Step::CombatDamage);
    assert_eq!(
        controller(&e, blocker),
        P1,
        "not yet: it is at end of combat"
    );

    lk::through_combat(&mut e);
    assert!(
        in_graveyard(&e, P0, tolarian_entrancer()).is_some(),
        "the 1/1 died to the Bears"
    );
    assert_eq!(
        controller(&e, blocker),
        P0,
        "the blocker is now the attacker's"
    );
    assert_eq!(
        controller(&e, bystander),
        P1,
        "a creature that did not block is not taken"
    );

    pass_until(&mut e, |e| {
        e.state().turn.number == 3
            && e.state().turn.step == Step::Main
            && matches!(e.pending(), Pending::Priority { .. })
    });
    assert_eq!(
        controller(&e, blocker),
        P0,
        "the control change is permanent"
    );
}

/// Tolarian Entrancer — "becomes blocked by", not "blocks": on the opponent's
/// turn the Entrancer blocks their Bears and nothing changes hands.
#[test]
fn tolarian_entrancer_takes_nothing_when_it_is_the_blocker() {
    let mut e = lk::game(SEED, &[tolarian_entrancer()], &[grizzly_bears()], &[], &[]);
    let entrancer = lk::obj(&e, P0, tolarian_entrancer());
    let their_bears = lk::obj(&e, P1, grizzly_bears());

    // Player 0 does not attack on their own turn.
    lk::attackers_offered(&mut e);
    e.apply(P0, PlayerAction::DeclareAttackers { attackers: vec![] })
        .unwrap();
    lk::attack(&mut e, &[their_bears]);
    lk::block(&mut e, &[(entrancer, their_bears)]);
    lk::through_combat(&mut e);

    assert_eq!(
        controller(&e, their_bears),
        P1,
        "blocking a creature does not take it"
    );
}
