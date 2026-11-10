//! `cards/sorceries/mv_6/bad_deal.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "47258c3e-b4a7-466d-9464-3924f75d9293"
fn bad_deal() -> CardIndex {
    card_index("47258c3e-b4a7-466d-9464-3924f75d9293")
}

/// Bad Deal: "You draw two cards and each opponent discards two cards. Each
/// player loses 2 life."
#[test]
fn bad_deal_draws_two_discards_two_and_costs_everyone_two_life() {
    let mut engine = table_with_their_hand(
        945,
        &[bad_deal()],
        &[swamp(); 6],
        &[],
        &[],
        &[forest(), plains(), island()],
    );
    let (hand_before, library_before) = (hand_len(&engine, me()), library_len(&engine, me()));
    cast_from_hand(&mut engine, me(), bad_deal());
    let asked = settle_asked(&mut engine);

    assert_eq!(
        hand_len(&engine, me()),
        hand_before - 1 + 2,
        "two drawn, the spell out"
    );
    assert_eq!(library_len(&engine, me()), library_before - 2);
    assert_eq!(hand_len(&engine, them()), 1, "they discarded two of three");
    assert_eq!(graveyard_len(&engine, them()), 2);
    assert_eq!(life_of(&engine, me()), 18, "the caster loses two too");
    assert_eq!(life_of(&engine, them()), 18);
    for question in &asked {
        let Pending::ChooseCards { player, .. } = question else {
            panic!("only discards are asked: {question:?}")
        };
        assert_eq!(*player, them(), "the opponent chooses what goes");
    }
}
