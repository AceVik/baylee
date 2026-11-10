//! `cards/creatures/mv_7/loamdragger_giant.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn loamdragger_giant() -> CardIndex {
    card_index("b7523824-8c3e-4735-8fbc-6a5f99d1e1ac")
}

/// Loamdragger Giant — a vanilla 7/6 for {4}{R/G}{R/G}{R/G}. Each hybrid
/// symbol takes red or green: two Mountains, a Forest and four Swamps pay it.
#[test]
fn loamdragger_giant_is_paid_with_red_or_green_for_each_hybrid() {
    let board = [lots(mountain(), 2), vec![forest()], lots(swamp(), 4)].concat();
    let mut engine = table7(&board, &[loamdragger_giant()], &[]);
    let giant = cast_and_resolve(&mut engine, loamdragger_giant());
    assert_body(&engine, giant, (7, 6), KeywordSet::default());
}

/// Seven Swamps are seven mana and none of it red or green, so the hybrid
/// part is not payable.
#[test]
fn loamdragger_giant_is_not_castable_without_red_or_green() {
    let mut engine = table7(&lots(swamp(), 7), &[loamdragger_giant()], &[]);
    tap_all_mana(&mut engine, PlayerId::new(0));
    assert!(!castable7(&engine, loamdragger_giant()));
}
