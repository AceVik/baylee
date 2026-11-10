//! `cards/creatures/mv_7/yoked_plowbeast.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn yoked_plowbeast() -> CardIndex {
    card_index("07237694-b5d5-4c92-a429-55e343b8dd1c")
}

/// Yoked Plowbeast — a vanilla 5/5 for {5}{W}{W}.
#[test]
fn yoked_plowbeast_is_a_five_five_for_seven() {
    let mut engine = table7(&lots(plains(), 7), &[yoked_plowbeast()], &[]);
    let beast = cast_and_resolve(&mut engine, yoked_plowbeast());
    assert_body(&engine, beast, (5, 5), KeywordSet::default());
}

/// Cycling {2}.
#[test]
fn yoked_plowbeast_cycles_for_two() {
    cycle_it(yoked_plowbeast(), &lots(plains(), 2), 0);
}
