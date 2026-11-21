//! `cards/creatures/mv_3/rushwood_legate.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn rushwood_legate() -> CardIndex {
    card_index("f3623759-bf3a-4846-9ef7-1b732ad4355f")
}

/// Rushwood Legate ({2}{G} 2/1 Dryad):
/// "If an opponent controls an Island and you control a Forest, you may cast
/// this spell without paying its mana cost."
#[test]
fn rushwood_legate_casts_for_free_only_when_opponent_controls_island_and_you_control_forest() {
    // Missing opponent Island (with tapped Forest so P0 has 0 mana available): not castable.
    let mut e_no_island = lk::game(SEED, &[forest()], &[plains()], &[rushwood_legate()], &[]);
    let f1 = lk::obj(&e_no_island, lk::P0, forest());
    e_no_island
        .dev_state_mut(lk::P0)
        .unwrap()
        .set_tapped(f1, true);
    e_no_island.refresh_offer();
    assert!(!lk::castable(&e_no_island, lk::P0, rushwood_legate()));

    // Missing your Forest (tapped Plains on P0, Island on P1): not castable.
    let mut e_no_forest = lk::game(SEED, &[plains()], &[island()], &[rushwood_legate()], &[]);
    let p1 = lk::obj(&e_no_forest, lk::P0, plains());
    e_no_forest
        .dev_state_mut(lk::P0)
        .unwrap()
        .set_tapped(p1, true);
    e_no_forest.refresh_offer();
    assert!(!lk::castable(&e_no_forest, lk::P0, rushwood_legate()));

    // Both conditions hold (even with P0's Forest tapped and 0 mana in pool): casts for {0}!
    let mut e = lk::game(SEED, &[forest()], &[island()], &[rushwood_legate()], &[]);
    let f = lk::obj(&e, lk::P0, forest());
    e.dev_state_mut(lk::P0).unwrap().set_tapped(f, true);
    e.refresh_offer();
    assert!(lk::castable(&e, lk::P0, rushwood_legate()));

    let card = in_hand(&e, lk::P0, rushwood_legate()).unwrap();
    e.apply(lk::P0, PlayerAction::CastSpell { card }).unwrap();
    lk::settle(&mut e);
    assert!(on_battlefield(&e, lk::P0, rushwood_legate()).is_some());
}
