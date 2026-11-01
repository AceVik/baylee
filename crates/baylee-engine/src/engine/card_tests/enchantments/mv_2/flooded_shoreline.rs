//! `cards/enchantments/mv_2/flooded_shoreline.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn flooded_shoreline() -> CardIndex {
    card_index("01fd889f-1839-44b9-b936-0e62c8f8a730")
}

/// Cards of `card` in `seat`'s hand.
fn in_hand_count(engine: &Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) -> usize {
    engine
        .state()
        .zones
        .list(ZoneLocation::Hand(seat))
        .iter()
        .filter(|id| {
            engine
                .state()
                .object(**id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
        })
        .count()
}

/// `Flooded Shoreline` ({U}{U}, Enchantment):
/// `{U}{U}, Return two Islands you control to their owner's hand: Return target
/// creature to its owner's hand.` Exactly two Islands go home as the cost (never the Mountain,
/// never more), the menu of targets is creatures only, and the creature is bounced.
#[test]
fn flooded_shoreline_returns_exactly_two_islands_to_bounce_target_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island(), mountain()])
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
    let mountain_id = on_battlefield(&engine, p0, mountain()).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: shoreline,
                ability_index: 0,
            },
        )
        .unwrap();
    let (mut islands_asked, mut targets_asked) = (0, 0);
    for _ in 0..8 {
        match engine.pending().clone() {
            Pending::ChooseTargets { options, .. } => {
                assert!(options.contains(&bear), "a creature is a target");
                assert!(
                    !options.contains(&shoreline) && !options.contains(&mountain_id),
                    "only creatures are on the menu"
                );
                targets_asked += 1;
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
                assert!(
                    !options.contains(&mountain_id) && !options.contains(&shoreline),
                    "the Islands are the cost: the Mountain is not on the menu"
                );
                islands_asked += 1;
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
    assert_eq!(targets_asked, 1, "one target question");
    assert_eq!(islands_asked, 2, "one Island chosen for each of the two");
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, GRIZZLY_BEARS).is_none());
    assert!(in_hand(&engine, p1, GRIZZLY_BEARS).is_some());
    assert_eq!(
        in_hand_count(&engine, p0, island()),
        2,
        "two Islands went back to the hand as the cost"
    );
    assert!(
        on_battlefield(&engine, p0, mountain()).is_some(),
        "the Mountain stays"
    );
    assert!(on_battlefield(&engine, p0, flooded_shoreline()).is_some());
}
