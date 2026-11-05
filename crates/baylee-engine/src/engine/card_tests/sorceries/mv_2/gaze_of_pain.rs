//! `cards/sorceries/mv_2/gaze_of_pain.rs`, played.
//!
//! Oracle:
//! - "Until end of turn, whenever a creature you control attacks and isn't blocked, you may choose to have it deal damage equal to its power to a target creature. If you do, it assigns no combat damage this turn."
//!
//! Both answers to the "you may" are played, as is an attacker that enters
//! only after Gaze of Pain has resolved and left the battlefield.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn gaze_of_pain() -> CardIndex {
    card_index("b3a479d4-3e36-496b-abf8-b83231628d76")
}

#[track_caller]
fn resolve_gaze_combat(
    engine: &mut Engine<RegistryLookup>,
    attacker: ObjectId,
    wall: ObjectId,
    yes: bool,
) {
    let (mut aimed, mut asked) = (false, false);
    for _ in 0..40 {
        if engine.state().turn.phase == Phase::SecondMain {
            break;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseTargets { options, .. } => {
                assert!(options.contains(&wall), "another creature: {options:?}");
                assert!(
                    options.contains(&attacker),
                    "the oracle says 'a target creature', not 'another': {options:?}"
                );
                aimed = true;
                lk::aim(engine, &[wall], &[]);
            }
            Pending::YesNo { player, .. } => {
                asked = true;
                engine.apply(player, PlayerAction::YesNo(yes)).unwrap();
            }
            other => panic!("unexpected pending {other:?}"),
        }
    }
    assert!(
        engine.state().turn.phase == Phase::SecondMain,
        "combat ended"
    );
    assert!(aimed, "the unblocked attacker's trigger asked for a target");
    assert!(asked, "the trigger asked whether to deal the damage");
}

/// Player 0 casts Gaze of Pain and attacks with Grizzly Bears; player 1
/// does not block. Returns at the second main phase, with the Wall.
fn attack_under_gaze(yes: bool) -> (Engine<RegistryLookup>, ObjectId) {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[index::WALL_OF_WOOD],
        &[gaze_of_pain()],
        &[],
    );
    let runner = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let wall = lk::obj(&engine, P1, index::WALL_OF_WOOD);
    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::Black, 1)],
    );
    lk::cast_at(&mut engine, P0, gaze_of_pain(), &[], &[]);
    assert!(in_graveyard(&engine, P0, gaze_of_pain()).is_some());

    let _ = lk::attack(&mut engine, &[runner]);
    lk::block(&mut engine, &[]);
    resolve_gaze_combat(&mut engine, runner, wall, yes);
    (engine, wall)
}

#[test]
fn gaze_of_pain_lets_an_unblocked_attacker_hit_a_creature_instead_of_the_player() {
    let (engine, wall) = attack_under_gaze(true);
    assert_eq!(
        lk::damage_on(&engine, wall),
        2,
        "the Bears dealt damage equal to their power to the target"
    );
    assert_eq!(
        lk::life(&engine, P1),
        20,
        "and assigned no combat damage this turn"
    );
}

#[test]
fn gaze_of_pain_declined_leaves_the_combat_damage_where_it_was() {
    let (engine, wall) = attack_under_gaze(false);
    assert_eq!(lk::damage_on(&engine, wall), 0, "no damage to the target");
    assert_eq!(lk::life(&engine, P1), 18, "the Bears' combat damage");
}

#[test]
fn gaze_of_pain_observes_a_creature_that_enters_after_it_resolves() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[index::WALL_OF_WOOD],
        &[gaze_of_pain(), index::RAGING_GOBLIN],
        &[],
    );
    let wall = lk::obj(&engine, P1, index::WALL_OF_WOOD);
    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::Black, 1)],
    );
    lk::cast_at(&mut engine, P0, gaze_of_pain(), &[], &[]);
    lk::float(&mut engine, P0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut engine, P0, index::RAGING_GOBLIN, &[], &[]);
    let goblin = lk::obj(&engine, P0, index::RAGING_GOBLIN);
    assert!(
        in_graveyard(&engine, P0, gaze_of_pain()).is_some(),
        "the observer is independent of its resolved sorcery source"
    );

    let _ = lk::attack(&mut engine, &[goblin]);
    lk::block(&mut engine, &[]);
    resolve_gaze_combat(&mut engine, goblin, wall, true);

    assert_eq!(
        lk::damage_on(&engine, wall),
        1,
        "the newly entered haste attacker deals its power to the target"
    );
    assert_eq!(
        lk::life(&engine, P1),
        20,
        "it assigns no combat damage after the optional effect is chosen"
    );
}

/// "Attacks and isn't blocked", "a creature you control", "until end of
/// turn": a blocked attacker, the opponent's attacker and a later turn's
/// unblocked attacker each ask nothing and deal their combat damage as usual.
#[test]
fn gaze_of_pain_ignores_blocked_attackers_opposing_attackers_and_later_turns() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[index::WALL_OF_WOOD, index::GRIZZLY_BEARS],
        &[gaze_of_pain()],
        &[],
    );
    let runner = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let wall = lk::obj(&engine, P1, index::WALL_OF_WOOD);
    let theirs = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::Black, 1)],
    );
    lk::cast_at(&mut engine, P0, gaze_of_pain(), &[], &[]);

    // Blocked: the Wall stops the Bears and nothing is asked.
    let _ = lk::attack(&mut engine, &[runner]);
    lk::block(&mut engine, &[(wall, runner)]);
    lk::through_combat(&mut engine);
    assert_eq!(lk::life(&engine, P1), 20);
    assert_eq!(lk::damage_on(&engine, wall), 2, "only combat damage");

    // The opponent's unblocked attacker is not "a creature you control".
    reach_their_main_phase(&mut engine, P1);
    let _ = lk::attack(&mut engine, &[theirs]);
    lk::block(&mut engine, &[]);
    lk::through_combat(&mut engine);
    assert_eq!(lk::life(&engine, P0), 18);

    // The next turn of the controller: the effect has ended.
    reach_their_main_phase(&mut engine, P0);
    let _ = lk::attack(&mut engine, &[runner]);
    lk::block(&mut engine, &[]);
    lk::through_combat(&mut engine);
    assert_eq!(lk::life(&engine, P1), 18, "no trigger, so no choice");
    assert_eq!(lk::damage_on(&engine, wall), 0);
}
