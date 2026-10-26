//! `cards/creatures/mv_1/skyshroud_ridgeback.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_cards_dsl::counters;

#[test]
fn skyshroud_ridgeback_enters_with_two_fade_counters_and_fades() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[card_index("34a160bc-5d7e-483c-b24c-ab30b7d313b0")])
        .battlefield(0, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(
        &mut engine,
        p0,
        card_index("34a160bc-5d7e-483c-b24c-ab30b7d313b0"),
    );
    pass_until(&mut engine, stack_is_empty);

    let ridgeback = on_battlefield(
        &engine,
        p0,
        card_index("34a160bc-5d7e-483c-b24c-ab30b7d313b0"),
    )
    .unwrap();
    assert_eq!(pt(&engine, ridgeback), (2, 3));
    assert_eq!(
        engine
            .state()
            .object(ridgeback)
            .unwrap()
            .counters
            .get(counters::FADE),
        2
    );

    for expected in [1, 0] {
        reach_their_main_phase(&mut engine, p1);
        reach_their_main_phase(&mut engine, p0);
        assert_eq!(
            engine
                .state()
                .object(ridgeback)
                .unwrap()
                .counters
                .get(counters::FADE),
            expected
        );
    }

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        on_battlefield(
            &engine,
            p0,
            card_index("34a160bc-5d7e-483c-b24c-ab30b7d313b0")
        )
        .is_none()
    );
    assert!(
        in_graveyard(
            &engine,
            p0,
            card_index("34a160bc-5d7e-483c-b24c-ab30b7d313b0")
        )
        .is_some()
    );
}
