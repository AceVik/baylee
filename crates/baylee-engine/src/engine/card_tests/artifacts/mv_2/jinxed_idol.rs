//! `cards/artifacts/mv_2/jinxed_idol.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn jinxed_idol() -> CardIndex {
    card_index("89b1eab9-4d24-4367-99f7-263349f69c3a")
}

/// Jinxed Idol (`89b1eab9-4d24-4367-99f7-263349f69c3a`) — {2} Artifact:
/// "At the beginning of your upkeep, this artifact deals 2 damage to you. Sacrifice a creature: Target opponent gains control of this artifact."
#[test]
fn jinxed_idol_sacrifices_creature_to_hand_control_to_opponent_and_damages_at_upkeep() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[jinxed_idol(), index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    assert_eq!(engine.state().players[0].life, 18);
    let idol = on_battlefield(&engine, P0, jinxed_idol()).unwrap();
    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    activate(&mut engine, P0, jinxed_idol(), 1);
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![P0]
                }
            )
            .is_err()
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(idol).unwrap().controller, P1);

    // Advance to P1's upkeep: Jinxed Idol deals 2 damage to P1.
    reach_their_main_phase(&mut engine, P1);
    assert_eq!(engine.state().players[1].life, 18);
    assert_eq!(engine.state().players[0].life, 18);
    reach_their_main_phase(&mut engine, P0);
    assert_eq!(engine.state().players[0].life, 18);
}
