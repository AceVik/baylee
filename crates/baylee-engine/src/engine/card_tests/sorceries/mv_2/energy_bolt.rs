//! `cards/sorceries/mv_2/energy_bolt.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::ENERGY_BOLT;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Energy Bolt (5c19db44-0eae-4d22-9bf4-7e6cdc6b9d8f) — {X}{R}{W} Sorcery:
/// "Choose one —
/// • This spell deals X damage to target player.
/// • Target player gains X life."
#[test]
fn energy_bolt_deals_x_damage_or_gains_x_life() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
            ],
        )
        .hand(0, &[ENERGY_BOLT, ENERGY_BOLT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    // Mode 0 with X = 3 targeting P1: deals 3 damage to P1.
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, ENERGY_BOLT);
    for _ in 0..10 {
        if stack_is_empty(&engine) && matches!(engine.pending(), Pending::Priority { .. }) {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseCastMode { options, .. } => {
                let slot = options
                    .iter()
                    .position(|o| matches!(o.kind, CastModeKind::Mode(0)))
                    .unwrap();
                engine.apply(P0, PlayerAction::ChooseMode(slot)).unwrap();
            }
            Pending::ChooseNumber { .. } => {
                engine.apply(P0, PlayerAction::ChooseNumber(3)).unwrap();
            }
            Pending::ChooseTargets { .. } => {
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
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected pending: {other:?}"),
        }
    }
    assert_eq!(engine.state().players[1].life, 17);

    // Mode 1 with X = 2 targeting P0: P0 gains 2 life.
    cast_with_floating(&mut engine, P0, ENERGY_BOLT);
    for _ in 0..10 {
        if stack_is_empty(&engine) && matches!(engine.pending(), Pending::Priority { .. }) {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseCastMode { options, .. } => {
                let slot = options
                    .iter()
                    .position(|o| matches!(o.kind, CastModeKind::Mode(1)))
                    .unwrap();
                engine.apply(P0, PlayerAction::ChooseMode(slot)).unwrap();
            }
            Pending::ChooseNumber { .. } => {
                engine.apply(P0, PlayerAction::ChooseNumber(2)).unwrap();
            }
            Pending::ChooseTargets { .. } => {
                engine
                    .apply(
                        P0,
                        PlayerAction::ChooseTargets {
                            objects: vec![],
                            players: vec![P0],
                        },
                    )
                    .unwrap();
            }
            Pending::ChoosePlayer { .. } => {
                engine.apply(P0, PlayerAction::ChoosePlayer(P0)).unwrap();
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected pending: {other:?}"),
        }
    }
    assert_eq!(engine.state().players[0].life, 22);
}
