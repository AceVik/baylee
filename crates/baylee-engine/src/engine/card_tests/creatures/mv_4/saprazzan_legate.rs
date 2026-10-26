//! `cards/creatures/mv_4/saprazzan_legate.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "d20b62fc-e248-4c84-87ac-7319b92531d4"
fn saprazzan_legate() -> CardIndex {
    card_index("d20b62fc-e248-4c84-87ac-7319b92531d4")
}

/// `Saprazzan Legate` (`Coverage::Partial`):
/// "Flying"
///
/// Verifies that `Saprazzan Legate` can be cast for `{3}{U}` and enters the
/// battlefield as a 1/3 creature with flying.
#[test]
fn saprazzan_legate_enters_with_flying() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(9614, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[saprazzan_legate()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, saprazzan_legate());
    pass_until(&mut engine, stack_is_empty);

    let legate = on_battlefield(&engine, p0, saprazzan_legate()).expect("P0 Saprazzan Legate");
    assert_eq!(pt(&engine, legate), (1, 3));
    assert!(keywords(&engine, legate).contains(KeywordSet::FLYING));
}
