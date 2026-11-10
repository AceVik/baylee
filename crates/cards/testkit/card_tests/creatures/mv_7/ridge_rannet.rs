//! `cards/creatures/mv_7/ridge_rannet.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn ridge_rannet() -> CardIndex {
    card_index("562aaf81-e70a-401d-a638-bb7ce4f40626")
}

/// Ridge Rannet — a vanilla 6/4 for {5}{R}{R}.
#[test]
fn ridge_rannet_is_a_six_four_for_seven() {
    let mut engine = table7(&lots(mountain(), 7), &[ridge_rannet()], &[]);
    let rannet = cast_and_resolve(&mut engine, ridge_rannet());
    assert_body(&engine, rannet, (6, 4), KeywordSet::default());
}

/// Cycling {2}.
#[test]
fn ridge_rannet_cycles_for_two() {
    cycle_it(ridge_rannet(), &lots(mountain(), 2), 0);
}
