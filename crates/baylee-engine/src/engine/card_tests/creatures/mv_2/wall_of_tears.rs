//! `cards/creatures/mv_2/wall_of_tears.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::engine::card_tests::partials_w1b_kit as kit;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn wall_of_tears() -> CardIndex {
    card_index("5825d718-6a32-4f4c-9b56-1a50051cfe28")
}

/// Wall of Tears (`5825d718-6a32-4f4c-9b56-1a50051cfe28`) — {1}{U} Creature — Wall (Implemented):
/// "Defender. Whenever this creature blocks a creature, return that creature to its owner's hand at end of combat."
#[test]
fn wall_of_tears_returns_blocked_creature_to_hand_at_end_of_combat() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[index::GRIZZLY_BEARS])
        .battlefield(1, &[wall_of_tears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let attacker = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    let wall = on_battlefield(&engine, P1, wall_of_tears()).unwrap();
    kit::declare_attack(&mut engine, &[attacker], P1);
    kit::drive(
        &mut engine,
        |e| e.state().turn.step == crate::turn::Step::CombatDamage && stack_is_empty(e),
        |_, p| match p {
            Pending::ChooseBlockers { .. } => Some(PlayerAction::DeclareBlockers {
                blockers: vec![(wall, attacker)],
            }),
            _ => None,
        },
    );

    assert!(
        on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_some(),
        "return waits until combat end"
    );
    lk::through_combat(&mut engine);
    assert!(on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_none());
    assert!(in_hand(&engine, P0, index::GRIZZLY_BEARS).is_some());
}

#[test]
fn defender_prevents_the_wall_itself_from_attacking() {
    let mut e = lk::game(SEED, &[wall_of_tears()], &[], &[], &[]);
    let wall = lk::obj(&e, P0, wall_of_tears());
    assert!(!lk::attackers_offered(&mut e).contains(&wall));
    assert!(
        e.apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(wall, Defender::Player(P1))]
            }
        )
        .is_err()
    );
}
