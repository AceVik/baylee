#[allow(clippy::wildcard_imports)]
use super::*;

fn sporogenesis() -> CardIndex {
    card_index("33005c63-fe9f-4051-b781-eacf1ca5eec6")
}

#[test]
fn sporogenesis_puts_fungus_counter_and_creates_saproling_when_it_dies() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[lightning_bolt()])
        .battlefield(0, &[sporogenesis(), gray_ogre(), mountain()])
        .start();
    keep_mulligans(&mut engine);

    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("ogre on bf");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim_at(&mut engine, p0, ogre);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    reach_main_phase(&mut engine, p0);

    assert_eq!(
        engine
            .state()
            .object(ogre)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::FUNGUS),
        1
    );

    cast_from_hand(&mut engine, p0, lightning_bolt());
    aim_at(&mut engine, p0, ogre);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(tokens_of(&engine, p0).len(), 1);
}
