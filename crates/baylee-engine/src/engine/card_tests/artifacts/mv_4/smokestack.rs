#[allow(clippy::wildcard_imports)]
use super::*;

fn smokestack() -> CardIndex {
    card_index("fda61506-ed79-47fd-badd-639185297913")
}

#[test]
fn smokestack_adds_soot_counter_and_forces_sacrifice_on_upkeep() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[smokestack(), gray_ogre()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("ogre on bf");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ogre],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    reach_main_phase(&mut engine, p0);

    let stack_id = on_battlefield(&engine, p0, smokestack()).expect("smokestack on bf");
    assert_eq!(
        engine
            .state()
            .object(stack_id)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::SOOT),
        1
    );
    assert!(on_battlefield(&engine, p0, gray_ogre()).is_none());

    let p1_ogre = on_battlefield(&engine, p1, gray_ogre()).expect("p1 ogre on bf");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![p1_ogre],
            },
        )
        .unwrap();
    reach_their_main_phase(&mut engine, p1);
    assert!(on_battlefield(&engine, p1, gray_ogre()).is_none());
}
