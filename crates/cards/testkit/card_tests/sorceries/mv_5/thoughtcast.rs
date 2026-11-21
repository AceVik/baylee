//! `cards/sorceries/mv_5/thoughtcast.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);

fn thoughtcast() -> CardIndex {
    card_index("cce9bbff-82dc-4b2f-addd-d6715588de20")
}

/// Thoughtcast (cce9bbff-82dc-4b2f-addd-d6715588de20) — {4}{U} Sorcery:
/// "Affinity for artifacts. Draw two cards."
#[test]
fn thoughtcast_costs_one_less_per_controlled_artifact_and_draws_two() {
    // 4 controlled non-mana artifacts reduce {4}{U} to {U} (opponent's artifact ignored).
    let mut engine = lk::game(
        SEED,
        &[
            island(),
            darksteel_pendant(),
            darksteel_pendant(),
            lightning_greaves(),
            lightning_greaves(),
        ],
        &[sol_ring()],
        &[thoughtcast()],
        &[],
    );
    let lib_before = library_size(&engine, P0);

    tap_all_mana(&mut engine, P0);
    assert_eq!(lk::pool_total(&engine, P0), 1, "only {{U}} in pool");
    cast_with_floating(&mut engine, P0, thoughtcast());
    lk::settle(&mut engine);

    assert_eq!(library_size(&engine, P0), lib_before - 2);
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(P0)).len(), 2);

    // With only 2 controlled artifacts (and 1 opposing artifact), {4}{U} -> {2}{U}, so {1}{U} is short.
    let mut short = lk::game(
        SEED,
        &[island(), island(), darksteel_pendant(), lightning_greaves()],
        &[darksteel_pendant(), lightning_greaves()],
        &[thoughtcast()],
        &[],
    );
    tap_all_mana(&mut short, P0);
    assert!(!lk::castable(&short, P0, thoughtcast()));
}
