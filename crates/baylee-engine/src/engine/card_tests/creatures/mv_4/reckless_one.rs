//! `cards/creatures/mv_4/reckless_one.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn reckless_one() -> CardIndex {
    card_index("6c8a8a7a-ee4f-4005-b6a2-fc418e396748")
}

#[test]
fn reckless_one_counts_every_goblin_and_attacks_with_haste() {
    let mut e = lk::game(
        SEED,
        &[index::RAGING_GOBLIN, grizzly_bears()],
        &[index::RAGING_GOBLIN],
        &[reckless_one()],
        &[],
    );
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 4)]);
    cast_with_floating(&mut e, lk::P0, reckless_one());
    lk::settle(&mut e);
    let reckless = lk::obj(&e, lk::P0, reckless_one());
    assert_eq!(pt(&e, reckless), (3, 3), "itself and both players' Goblins");
    let opposing_goblin = lk::obj(&e, lk::P1, index::RAGING_GOBLIN);
    kill(&mut e, opposing_goblin);
    assert_eq!(
        pt(&e, reckless),
        (2, 2),
        "the count updates when a Goblin dies"
    );
    lk::attack(&mut e, &[reckless]);
    assert!(e.state().combat.is_attacking(reckless));
}
