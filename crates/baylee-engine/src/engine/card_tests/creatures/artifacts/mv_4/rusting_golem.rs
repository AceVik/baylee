//! `cards/artifacts/mv_4/rusting_golem.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn rusting_golem_power_and_toughness_equal_fade_counters_on_it() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[card_index("688c232f-385b-4bd5-aed0-e55059111fb3")])
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(
        &mut engine,
        p0,
        card_index("688c232f-385b-4bd5-aed0-e55059111fb3"),
    );
    pass_until(&mut engine, stack_is_empty);

    let golem = on_battlefield(
        &engine,
        p0,
        card_index("688c232f-385b-4bd5-aed0-e55059111fb3"),
    )
    .expect("Rusting Golem");
    assert_eq!(pt(&engine, golem), (5, 5));

    // After p0's next upkeep removes 1 fade counter, it becomes 4/4.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(pt(&engine, golem), (4, 4));
}
