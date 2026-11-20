//! `cards/sorceries/mv_5/temporal_fissure.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;
use baylee_engine::choice::TargetPrompt;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Temporal Fissure — {4}{U} Sorcery:
/// "Return target permanent to its owner's hand.
/// Storm (When you cast this spell, copy it for each spell cast before it this turn. You may choose new targets for the copies.)"
#[test]
fn temporal_fissure_bounces_any_permanent_and_retargets_storm_copy() {
    let fissure = card_index("8a9525f4-74a6-44f0-a1b1-d57937d7783b");
    let mut engine = Duel::new(SEED, forest())
        .battlefield(1, &[index::GRIZZLY_BEARS, forest()])
        .hand(0, &[index::DARK_RITUAL, fissure])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();
    let opp_forest = on_battlefield(&engine, P1, forest()).unwrap();
    {
        let state = engine.dev_state_mut(P0).unwrap();
        state.players[0].mana_pool.add(ManaColor::Black, 1);
        state.players[0].mana_pool.add(ManaColor::Blue, 5);
    }
    engine.refresh_offer();

    // 1 prior spell -> Storm creates 1 copy.
    cast_with_floating(&mut engine, P0, index::DARK_RITUAL);
    pass_until(&mut engine, stack_is_empty);

    cast_with_floating(&mut engine, P0, fissure);
    let options = pass_until_targets(&mut engine, P0);
    assert!(
        options.contains(&bears) && options.contains(&opp_forest),
        "Temporal Fissure must be able to target both creatures and lands: {options:?}"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();

    let mut retargeted = 0usize;
    for _ in 0..30 {
        if stack_is_empty(&engine) && matches!(engine.pending(), Pending::Priority { .. }) {
            break;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseTargets {
                player,
                reason: TargetPrompt::Retarget { .. },
                ..
            } => {
                assert_eq!(player, P0);
                retargeted += 1;
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseTargets {
                            objects: vec![opp_forest],
                            players: vec![],
                        },
                    )
                    .unwrap();
            }
            other => panic!("unexpected pending during Temporal Fissure: {other:?}"),
        }
    }

    assert_eq!(retargeted, 1);
    assert_eq!(engine.state().per_turn.spells_cast, vec![2, 0]);
    assert!(on_battlefield(&engine, P1, index::GRIZZLY_BEARS).is_none());
    assert!(on_battlefield(&engine, P1, forest()).is_none());
    assert!(in_hand(&engine, P1, index::GRIZZLY_BEARS).is_some());
    assert!(in_hand(&engine, P1, forest()).is_some());
}
