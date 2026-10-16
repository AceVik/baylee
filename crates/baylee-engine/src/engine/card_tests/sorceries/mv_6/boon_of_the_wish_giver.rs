//! `cards/sorceries/mv_6/boon_of_the_wish_giver.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "1720d38a-c3c1-4294-85c1-a91fc7179d6f"
fn boon_of_the_wish_giver() -> CardIndex {
    card_index("1720d38a-c3c1-4294-85c1-a91fc7179d6f")
}

/// Boon of the Wish-Giver: "Draw four cards."
#[test]
fn boon_of_the_wish_giver_draws_four() {
    let mut engine = table(946, &[boon_of_the_wish_giver()], &[island(); 6], &[], &[]);
    let (hand_before, library_before) = (hand_len(&engine, me()), library_len(&engine, me()));
    cast_plain(&mut engine, boon_of_the_wish_giver());
    assert_eq!(hand_len(&engine, me()), hand_before - 1 + 4);
    assert_eq!(library_len(&engine, me()), library_before - 4);
}

/// Cycling {1}: one mana, the card discarded, one card drawn.
#[test]
fn boon_of_the_wish_giver_cycles_for_one() {
    let mut engine = table(947, &[boon_of_the_wish_giver()], &[island()], &[], &[]);
    let card = in_hand(&engine, me(), boon_of_the_wish_giver()).expect("in hand");
    let (hand_before, library_before) = (hand_len(&engine, me()), library_len(&engine, me()));
    tap_all_mana(&mut engine, me());
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: card,
                ability_index: 0,
            },
        )
        .expect("cycling is offered from the hand");
    settle(&mut engine);
    assert!(in_graveyard(&engine, me(), boon_of_the_wish_giver()).is_some(), "discarded");
    assert_eq!(hand_len(&engine, me()), hand_before, "one out, one in");
    assert_eq!(library_len(&engine, me()), library_before - 1, "one card, not four");
}
