//! `cards/sorceries/mv_6/mindculling.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "955a7f4d-7100-4d8b-818c-be8651d0ebe6"
fn mindculling() -> CardIndex {
    card_index("955a7f4d-7100-4d8b-818c-be8651d0ebe6")
}

/// Mindculling: "You draw two cards and target opponent discards two cards."
///
/// Only an opponent is on the menu; they discard two of three, the caster
/// draws two.
#[test]
fn mindculling_draws_two_and_makes_the_opponent_discard_two() {
    let mut engine = table_with_their_hand(
        967,
        &[mindculling()],
        &[island(); 6],
        &[],
        &[],
        &[forest(), plains(), island()],
    );
    let (hand_before, library_before) = (hand_len(&engine, me()), library_len(&engine, me()));
    let (_, players, asked) = cast_at_player_asked(&mut engine, mindculling(), them());
    assert!(players.contains(&them()), "the opponent: {players:?}");
    assert!(!players.contains(&me()), "not the caster: {players:?}");
    assert_eq!(hand_len(&engine, me()), hand_before - 1 + 2);
    assert_eq!(library_len(&engine, me()), library_before - 2);
    assert_eq!(hand_len(&engine, them()), 1, "three less two");
    assert_eq!(graveyard_len(&engine, them()), 2);
    for question in &asked {
        let Pending::ChooseCards { player, .. } = question else {
            panic!("only discards are asked: {question:?}")
        };
        assert_eq!(*player, them());
    }
}
