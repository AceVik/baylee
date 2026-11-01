//! `cards/creatures/mv_1/stampede_driver.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn stampede_driver_pumps_your_creatures_and_grants_trample() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(
            0,
            &[card_index("3c652a31-3b56-4050-9613-d0481e54dfd7"), forest()],
        )
        .battlefield(0, &[forest(), forest(), index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(
        &mut engine,
        p0,
        card_index("3c652a31-3b56-4050-9613-d0481e54dfd7"),
    );
    pass_until(&mut engine, stack_is_empty);

    let driver = on_battlefield(
        &engine,
        p0,
        card_index("3c652a31-3b56-4050-9613-d0481e54dfd7"),
    )
    .unwrap();
    let bears = on_battlefield(&engine, p0, index::GRIZZLY_BEARS).unwrap();

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    tap_all_mana_but(
        &mut engine,
        p0,
        Some(card_index("3c652a31-3b56-4050-9613-d0481e54dfd7")),
    );
    activate(
        &mut engine,
        p0,
        card_index("3c652a31-3b56-4050-9613-d0481e54dfd7"),
        0,
    );

    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected discard cost choice");
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, driver));
    assert_eq!(pt(&engine, driver), (2, 2));
    assert_eq!(pt(&engine, bears), (3, 3));
    assert!(keywords(&engine, bears).contains(KeywordSet::TRAMPLE));
}
