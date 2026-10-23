#[allow(clippy::wildcard_imports)]
use super::*;

fn argothian_wurm() -> CardIndex {
    card_index("f8aa1893-60e5-439e-9316-872a9f6b6c94")
}

#[test]
fn argothian_wurm_put_on_top_of_library_when_opponent_sacrifices_land() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[argothian_wurm()])
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let opp_land = on_battlefield(&engine, p1, forest()).expect("seated");
    cast_from_hand(&mut engine, p0, argothian_wurm());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });

    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![opp_land],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, forest()).is_none());
    assert!(in_graveyard(&engine, p1, forest()).is_some());
    assert!(on_battlefield(&engine, p0, argothian_wurm()).is_none());
    let top = *engine.state().zones.list(ZoneLocation::Library(p0)).last().unwrap();
    assert_eq!(
        engine.state().object(top).unwrap().card.unwrap().index,
        argothian_wurm()
    );
}
