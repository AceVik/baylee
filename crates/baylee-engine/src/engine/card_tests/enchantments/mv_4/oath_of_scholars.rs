use super::*;

fn oath_of_scholars() -> CardIndex {
    card_index("da4a5129-b005-4fc6-b175-0416627881db")
}

#[test]
fn oath_of_scholars_discards_hand_and_draws_three_on_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[oath_of_scholars()])
        .hand(0, &[island()])
        .start();
    keep_mulligans(&mut engine);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(p0)).len(), 3);
    assert!(in_graveyard(&engine, p0, island()).is_some());
}
