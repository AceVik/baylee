//! `cards/instants/mv_2/fling.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn fling() -> CardIndex {
    card_index("24227761-b50e-4b9e-93a2-e82d053b3e3d")
}

/// Fling (`24227761-b50e-4b9e-93a2-e82d053b3e3d`) — {1}{R} Instant (Implemented):
/// "As an additional cost to cast this spell, sacrifice a creature.
/// This spell deals damage equal to the sacrificed creature's power to any target."
#[test]
fn fling_sacrifices_creature_and_deals_its_power_in_damage() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), index::HILL_GIANT])
        .hand(0, &[fling()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let giant = on_battlefield(&engine, P0, index::HILL_GIANT).unwrap();
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, fling());
    for _ in 0..5 {
        match engine.pending().clone() {
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
            Pending::ChooseCards { .. } => {
                engine
                    .apply(
                        P0,
                        PlayerAction::ChooseObjects {
                            objects: vec![giant],
                        },
                    )
                    .unwrap();
            }
            Pending::Priority { .. } => break,
            other => panic!("unexpected pending: {other:?}"),
        }
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P0, index::HILL_GIANT).is_some());
    assert_eq!(engine.state().players[1].life, 17);
}

#[test]
fn opposing_creature_cannot_pay_the_additional_sacrifice() {
    let mut e = lk::game(SEED, &[], &[index::HILL_GIANT], &[fling()], &[]);
    lk::float(&mut e, P0, &[(ManaColor::Red, 2)]);
    let spell = in_hand(&e, P0, fling()).unwrap();
    let Pending::Priority { legal, .. } = e.pending() else {
        unreachable!()
    };
    assert!(!legal.castable.contains(&spell) && !legal.payable.contains(&spell));
}
