//! `cards/lands/tapland/hickory_woodlot.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The five Mercadian Masques depletion lands: "This land enters tapped
/// with two depletion counters on it."
///
/// One sentence the Vivid cycle also prints, with one word changed, and the
/// word is the whole test. A depletion counter is not a charge counter —
/// it is `counters::DEPLETION`, an id assigned in the DSL rather than a
/// rules kind — so both are asserted on every land: two of the one and none
/// of the other. A reader that took the number out of the phrase and threw
/// the noun away would pass the first assertion and fail the second, which
/// is exactly the failure that would otherwise have shipped five lands
/// spending counters they never arrived with.
#[test]
fn the_depletion_cycle_arrives_tapped_with_counters_of_its_own_kind() {
    let p0 = PlayerId::new(0);
    let cycle = [
        hickory_woodlot(),
        peat_bog(),
        remote_farm(),
        sandstone_needle(),
        saprazzan_skerry(),
    ];
    let mut engine = Duel::new(551, forest()).hand(0, &cycle).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    for (n, card) in cycle.into_iter().enumerate() {
        let land = play_land(&mut engine, p0, card);
        assert!(
            entered_tapped(&engine, land),
            "depletion land {n}: the sentence says tapped"
        );
        assert_eq!(
            counters_on(&engine, land, counters::DEPLETION),
            2,
            "depletion land {n}: and the same sentence says two depletion \
             counters"
        );
        assert_eq!(
            counters_on(&engine, land, CounterKind::Charge),
            0,
            "depletion land {n}: depletion and charge are two counters, and \
             this is the assertion a reader that dropped the noun fails"
        );
        cross_into_the_next_own_main(&mut engine, p0);
    }
}
