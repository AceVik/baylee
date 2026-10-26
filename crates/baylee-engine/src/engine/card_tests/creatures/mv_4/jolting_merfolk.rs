//! `cards/creatures/mv_4/jolting_merfolk.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_cards_dsl::counters;
use baylee_core::generated::index;

#[test]
fn jolting_merfolk_enters_with_four_fade_counters_and_removes_one_to_tap_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[card_index("3b71aeee-a4a8-46d3-9973-06db14d8bf6c")])
        .battlefield(0, &[island(), island(), island(), island()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bears = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).expect("Grizzly Bears");
    cast_from_hand(
        &mut engine,
        p0,
        card_index("3b71aeee-a4a8-46d3-9973-06db14d8bf6c"),
    );
    pass_until(&mut engine, stack_is_empty);
    let merfolk = on_battlefield(
        &engine,
        p0,
        card_index("3b71aeee-a4a8-46d3-9973-06db14d8bf6c"),
    )
    .expect("Jolting Merfolk");
    assert_eq!(
        engine
            .state()
            .object(merfolk)
            .unwrap()
            .counters
            .get(counters::FADE),
        4,
        "enters with 4 fade counters"
    );
    activate(
        &mut engine,
        p0,
        card_index("3b71aeee-a4a8-46d3-9973-06db14d8bf6c"),
        1,
    );
    aim_at(&mut engine, p0, bears);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine
            .state()
            .object(merfolk)
            .unwrap()
            .counters
            .get(counters::FADE),
        3
    );
    assert!(is_tapped(&engine, bears));
}
