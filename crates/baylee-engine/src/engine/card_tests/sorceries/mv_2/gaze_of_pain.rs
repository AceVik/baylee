//! `cards/sorceries/mv_2/gaze_of_pain.rs`, played.
//!
//! Oracle:
//! - "Until end of turn, whenever a creature you control attacks and isn't blocked, you may choose to have it deal damage equal to its power to a target creature. If you do, it assigns no combat damage this turn."
//!
//! The card is `Coverage::Partial`: the trigger is granted to the creatures
//! its controller controls as it resolves, not to one that arrives later
//! that turn. Both answers to the "you may" are played here.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn gaze_of_pain() -> CardIndex {
    card_index("b3a479d4-3e36-496b-abf8-b83231628d76")
}

/// Player 0 casts Gaze of Pain and attacks with Grizzly Bears; player 1
/// does not block. The trigger is aimed at player 1's Wall of Wood and its
/// "you may" is answered `yes`. Returns at the second main phase, with the
/// Wall.
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
                assert!(options.contains(&wall), "a target creature: {options:?}");
                aimed = true;
                lk::aim(&mut engine, &[wall], &[]);
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
