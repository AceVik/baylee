#[allow(clippy::wildcard_imports)]
use super::*;

fn symbiosis() -> CardIndex {
    card_index("787b494b-731b-4e99-8e46-157bc86f6eea")
}

#[test]
fn symbiosis_pumps_two_target_creatures() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[symbiosis()])
        .battlefield(0, &[gray_ogre(), forest(), forest()])
        .battlefield(1, &[wild_elephant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("ogre on bf");
    let elephant = on_battlefield(&engine, p1, wild_elephant()).expect("elephant on bf");
    cast_from_hand(&mut engine, p0, symbiosis());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![ogre, elephant],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, ogre), (4, 4));
    assert_eq!(pt(&engine, elephant), (5, 5));
}
