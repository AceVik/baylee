//! `cards/sorceries/mv_6/thassa_s_bounty.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "c43425c1-ad01-4088-a061-f691fa2f6c20"
fn thassa_s_bounty() -> CardIndex {
    card_index("c43425c1-ad01-4088-a061-f691fa2f6c20")
}

/// Thassa's Bounty: "Draw three cards. Target player mills three cards."
///
/// The opponent is named: their library loses three to their graveyard, and the
/// caster draws three.
#[test]
fn thassa_s_bounty_draws_three_and_mills_the_target_player() {
    let mut engine = table(978, &[thassa_s_bounty()], &[island(); 6], &[], &[]);
    let (hand_before, mine) = (hand_len(&engine, me()), library_len(&engine, me()));
    let theirs = library_len(&engine, them());
    let (_, players) = cast_at_player(&mut engine, thassa_s_bounty(), them());
    assert!(
        players.contains(&me()) && players.contains(&them()),
        "any player: {players:?}"
    );
    assert_eq!(hand_len(&engine, me()), hand_before - 1 + 3);
    assert_eq!(library_len(&engine, me()), mine - 3, "I only drew");
    assert_eq!(
        library_len(&engine, them()),
        theirs - 3,
        "they milled three"
    );
    assert_eq!(graveyard_len(&engine, them()), 3);
}
