//! `cards/lands/tapland/vivid_creek.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The Vivid cycle: "This land enters tapped with two charge counters on it."
///
/// Five cards one rule wrote, played one per turn, and the reason they are
/// one test is the reason the convoke lands are: a difference between them
/// would be a difference in the printing, not in the code.
///
/// What is being proved is that **one sentence is two replacement effects**
/// (CR 614.1c). `EnterModifier` is a list rather than a shape, so "tapped"
/// and "with two charge counters" are two entries applied to the same event,
/// and a reader that took only the first would leave five ordinary taplands
/// in the pool with an ability nothing could ever afford.
#[test]
fn the_vivid_cycle_arrives_tapped_and_brings_two_counters_with_it() {
    let p0 = PlayerId::new(0);
    let cycle = [
        vivid_crag(),
        vivid_creek(),
        vivid_grove(),
        vivid_marsh(),
        vivid_meadow(),
    ];
    let mut engine = Duel::new(541, forest()).hand(0, &cycle).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    for (n, card) in cycle.into_iter().enumerate() {
        let land = play_land(&mut engine, p0, card);
        assert!(
            entered_tapped(&engine, land),
            "vivid land {n}: the sentence says tapped"
        );
        assert_eq!(
            counters_on(&engine, land, CounterKind::Charge),
            2,
            "vivid land {n}: and the same sentence says two charge counters"
        );
        cross_into_the_next_own_main(&mut engine, p0);
    }
}
