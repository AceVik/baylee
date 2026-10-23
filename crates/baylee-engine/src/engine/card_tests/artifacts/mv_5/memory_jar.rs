#[allow(clippy::wildcard_imports)]
use super::*;

fn memory_jar() -> CardIndex {
    card_index("eb9c9a34-88a8-4afe-a053-806f6b29834f")
}

#[test]
fn memory_jar_discards_hands_and_draws_seven_cards() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains(), plains()])
        .hand(0, &[memory_jar(), gray_ogre()])
        .hand(1, &[plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, memory_jar());
    pass_until(&mut engine, stack_is_empty);

    let jar = on_battlefield(&engine, p0, memory_jar()).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: jar,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, memory_jar()).is_none());
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(p0)).len(), 7);
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(p1)).len(), 7);
}
