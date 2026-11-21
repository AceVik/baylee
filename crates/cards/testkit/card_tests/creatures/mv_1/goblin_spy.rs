//! `cards/creatures/mv_1/goblin_spy.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn goblin_spy() -> CardIndex {
    card_index("5f1e8aa2-6b85-4e2e-92e4-876470447d6f")
}

/// Goblin Spy ({R} 1/1 Goblin Rogue):
/// "Play with the top card of your library revealed."
#[test]
fn goblin_spy_reveals_only_your_library_top_while_on_battlefield() {
    let mut e = lk::game(SEED, &[mountain()], &[], &[goblin_spy(), index::SHOCK], &[]);
    assert!(!e.state().library_top_revealed(lk::P0));
    assert!(!e.state().library_top_revealed(lk::P1));

    cast_from_hand(&mut e, lk::P0, goblin_spy());
    lk::settle(&mut e);
    let spy = lk::obj(&e, lk::P0, goblin_spy());

    assert!(e.state().library_top_revealed(lk::P0));
    assert!(!e.state().library_top_revealed(lk::P1));

    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut e, lk::P0, index::SHOCK, &[spy], &[]);
    assert!(!e.state().library_top_revealed(lk::P0));
}
