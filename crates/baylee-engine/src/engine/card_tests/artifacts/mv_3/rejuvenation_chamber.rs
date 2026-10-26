//! `cards/artifacts/mv_3/rejuvenation_chamber.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn rejuvenation_chamber_enters_with_two_fade_counters_and_taps_for_two_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[card_index("c1ebe189-db71-47cd-a763-8cb8c305383a")])
        .battlefield(0, &[forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(
        &mut engine,
        p0,
        card_index("c1ebe189-db71-47cd-a763-8cb8c305383a"),
    );
    pass_until(&mut engine, stack_is_empty);

    let chamber = on_battlefield(
        &engine,
        p0,
        card_index("c1ebe189-db71-47cd-a763-8cb8c305383a"),
    )
    .expect("chamber");
    assert_eq!(
        engine
            .state()
            .object(chamber)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::FADE),
        2
    );

    activate(
        &mut engine,
        p0,
        card_index("c1ebe189-db71-47cd-a763-8cb8c305383a"),
        1,
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 22);
}

#[test]
fn rejuvenation_chamber_fades_out_when_no_fade_counter_remains() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[card_index("c1ebe189-db71-47cd-a763-8cb8c305383a")])
        .battlefield(0, &[forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(
        &mut engine,
        p0,
        card_index("c1ebe189-db71-47cd-a763-8cb8c305383a"),
    );
    pass_until(&mut engine, stack_is_empty);

    // Upkeep 1 removes fade counter (2 -> 1).
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        on_battlefield(
            &engine,
            p0,
            card_index("c1ebe189-db71-47cd-a763-8cb8c305383a")
        )
        .is_some()
    );

    // Upkeep 2 removes fade counter (1 -> 0).
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        on_battlefield(
            &engine,
            p0,
            card_index("c1ebe189-db71-47cd-a763-8cb8c305383a")
        )
        .is_some()
    );

    // Upkeep 3 has no fade counter left, so Rejuvenation Chamber is sacrificed.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        on_battlefield(
            &engine,
            p0,
            card_index("c1ebe189-db71-47cd-a763-8cb8c305383a")
        )
        .is_none(),
        "sacrificed when no fade counter could be removed at upkeep"
    );
}
