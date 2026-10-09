//! `cards/enchantments/mv_2/raging_river.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Raging River resolves onto the battlefield and asks nothing until its
/// controller attacks. Its piles are played in `engine::pile_block_tests`;
/// this file waits for the card's own test (the stub the hook asks for).
#[test]
fn raging_river_is_cast_and_waits_for_an_attack() {
    let raging_river = card_index("a2310312-6e1e-4e34-a351-9aef499a810f");
    assert_eq!(
        cast_saying_nothing(raging_river, mountain(), 2),
        Zone::Battlefield,
        "Raging River"
    );
}
