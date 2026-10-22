//! `cards/enchantments/mv_2/flooded_shoreline.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn flooded_shoreline() -> CardIndex {
    card_index("01fd889f-1839-44b9-b936-0e62c8f8a730")
}

/// `Flooded Shoreline` ({U}{U}, Enchantment, `Coverage::Implemented`):
/// `{U}{U}, Return two Islands you control to their owner's hand: Return target
/// creature to its owner's hand.`
#[test]
fn flooded_shoreline_returns_two_islands_to_bounce_target_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .hand(0, &[flooded_shoreline()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, flooded_shoreline());
    pass_until(&mut engine, stack_is_empty);
    let shoreline = on_battlefield(&engine, p0, flooded_shoreline()).unwrap();
    let bear = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: shoreline,
                ability_index: 0,
            },
        )
        .unwrap();
    for _ in 0..8 {
        match engine.pending().clone() {
            Pending::ChooseTargets { .. } => {
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseTargets {
                            objects: vec![bear],
                            players: vec![],
                        },
                    )
                    .unwrap();
            }
            Pending::ChooseCards { options, .. } => {
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![options[0]],
                        },
                    )
                    .unwrap();
            }
            Pending::Priority { .. } => break,
            other => panic!("unexpected pending during Flooded Shoreline activation: {other:?}"),
        }
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, GRIZZLY_BEARS).is_none());
    assert!(in_hand(&engine, p1, GRIZZLY_BEARS).is_some());
}
