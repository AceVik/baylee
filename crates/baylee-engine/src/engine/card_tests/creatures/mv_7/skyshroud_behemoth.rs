//! `cards/creatures/mv_7/skyshroud_behemoth.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_cards_dsl::counters;

#[test]
fn skyshroud_behemoth_enters_tapped_with_two_fade_counters_and_fades() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[card_index("77bee041-6f96-44e3-a18c-b52e508b9bdf")])
        .battlefield(0, &[forest(); 7])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(
        &mut engine,
        p0,
        card_index("77bee041-6f96-44e3-a18c-b52e508b9bdf"),
    );
    pass_until(&mut engine, stack_is_empty);

    let behemoth = on_battlefield(
        &engine,
        p0,
        card_index("77bee041-6f96-44e3-a18c-b52e508b9bdf"),
    )
    .unwrap();
    assert!(is_tapped(&engine, behemoth));
    assert_eq!(
        engine
            .state()
            .object(behemoth)
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
                .object(behemoth)
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
            card_index("77bee041-6f96-44e3-a18c-b52e508b9bdf")
        )
        .is_none()
    );
    assert!(
        in_graveyard(
            &engine,
            p0,
            card_index("77bee041-6f96-44e3-a18c-b52e508b9bdf")
        )
        .is_some()
    );
}
