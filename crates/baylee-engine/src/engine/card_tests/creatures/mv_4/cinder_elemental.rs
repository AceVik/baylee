//! `cards/creatures/mv_4/cinder_elemental.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Cinder Elemental — {3}{R} 2/2 Creature — Elemental:
/// "{X}{R}, {T}, Sacrifice this creature: It deals X damage to any target."
#[test]
fn cinder_elemental_sacrifices_to_deal_x_damage_to_any_target() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[index::CINDER_ELEMENTAL])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    engine.dev_state_mut(P0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Red, 5);
    engine.refresh_offer();

    activate(&mut engine, P0, index::CINDER_ELEMENTAL, 0);
    for _ in 0..4 {
        match engine.pending().clone() {
            Pending::ChooseNumber { .. } => {
                engine.apply(P0, PlayerAction::ChooseNumber(4)).unwrap();
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
            _ => break,
        }
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, index::CINDER_ELEMENTAL).is_none());
    assert!(in_graveyard(&engine, P0, index::CINDER_ELEMENTAL).is_some());
    assert_eq!(engine.state().players[1].life, 16);
}
