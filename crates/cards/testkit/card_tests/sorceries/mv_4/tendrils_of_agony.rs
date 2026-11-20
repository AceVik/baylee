//! `cards/sorceries/mv_4/tendrils_of_agony.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;
use baylee_engine::choice::TargetPrompt;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Tendrils of Agony — {2}{B}{B} Sorcery:
/// "Target player loses 2 life and you gain 2 life.
/// Storm (When you cast this spell, copy it for each spell cast before it this turn. You may choose new targets for the copies.)"
#[test]
fn tendrils_of_agony_drains_per_storm_copy_and_allows_retargeting_a_copy() {
    let tendrils = card_index("78db3190-59bc-4033-95bc-32d8ed872b6a");
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[index::DARK_RITUAL, tendrils])
        .hand(1, &[index::DARK_RITUAL])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    {
        let state = engine.dev_state_mut(P0).unwrap();
        state.players[0].mana_pool.add(ManaColor::Black, 5);
        state.players[1].mana_pool.add(ManaColor::Black, 1);
    }
    engine.refresh_offer();

    // Two prior spells -> 2 Storm copies + 1 original.
    cast_with_floating(&mut engine, P0, index::DARK_RITUAL);
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    cast_with_floating(&mut engine, P1, index::DARK_RITUAL);
    pass_until(&mut engine, stack_is_empty);

    cast_with_floating(&mut engine, P0, tendrils);
    match engine.pending().clone() {
        Pending::ChoosePlayer { player, .. } => {
            assert_eq!(player, P0);
            engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
        }
        Pending::ChooseTargets { player, .. } => {
            assert_eq!(player, P0);
            engine
                .apply(
                    P0,
                    PlayerAction::ChooseTargets {
                        objects: vec![],
                        players: vec![P1],
                    },
                )
                .unwrap();
        }
        other => panic!("expected player target choice for Tendrils of Agony, got {other:?}"),
    }

    // Retarget first Storm copy to P0 (net 0 for P0); keep second copy + original on P1.
    let mut retargeted = 0usize;
    for _ in 0..40 {
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
                let players = if retargeted == 0 { vec![P0] } else { vec![] };
                retargeted += 1;
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseTargets {
                            objects: vec![],
                            players,
                        },
                    )
                    .unwrap();
            }
            other => panic!("unexpected pending during Tendrils of Agony: {other:?}"),
        }
    }

    assert_eq!(
        retargeted, 2,
        "Storm created two copies for two prior casts"
    );
    assert_eq!(engine.state().per_turn.spells_cast, vec![2, 1]);
    // Copy 1 on P0: -2/+2 (net 0). Copy 2 + original on P1: P1 loses 4 (20->16), P0 gains 4 (20->24).
    assert_eq!(engine.state().players[0].life, 24);
    assert_eq!(engine.state().players[1].life, 16);
}
