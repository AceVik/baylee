//! `cards/sorceries/mv_6/rapturous_moment.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "c997afb7-82cb-4579-885e-3628cb1b51e4"
fn rapturous_moment() -> CardIndex {
    card_index("c997afb7-82cb-4579-885e-3628cb1b51e4")
}

/// Rapturous Moment: "Draw three cards, then discard two cards. Add
/// {U}{U}{R}{R}{R}."
#[test]
fn rapturous_moment_loots_three_for_two_and_adds_five_mana() {
    let lands = [vec![island(); 5], vec![mountain()]].concat();
    let mut engine = table(970, &[rapturous_moment()], &lands, &[], &[]);
    let (hand_before, library_before) = (hand_len(&engine, me()), library_len(&engine, me()));
    cast_from_hand(&mut engine, me(), rapturous_moment());
    let asked = settle_asked(&mut engine);

    assert!(
        asked
            .iter()
            .any(|q| matches!(q, Pending::ChooseCards { .. })),
        "a discard is asked: {asked:?}"
    );
    assert_eq!(
        library_len(&engine, me()),
        library_before - 3,
        "three drawn"
    );
    assert_eq!(hand_len(&engine, me()), hand_before - 1 + 3 - 2);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 2);
    assert_eq!(pool.available(ManaColor::Red), 3);
    assert_eq!(pool.total(), 5, "and nothing else");
}
