//! `cards/sorceries/mv_4/spontaneous_generation.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "d5b021e3-a0c1-471b-9088-a76816806354"
fn spontaneous_generation() -> CardIndex {
    card_index("d5b021e3-a0c1-471b-9088-a76816806354")
}

/// `Spontaneous Generation` (`Coverage::Generated`):
/// "Create a 1/1 green Saproling creature token for each card in your hand."
///
/// Verifies that `Spontaneous Generation` counts the cards remaining in your
/// hand when it resolves and creates that many 1/1 green Saproling tokens.
#[test]
fn spontaneous_generation_creates_saprolings_equal_to_cards_in_hand_on_resolution() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(9604, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[spontaneous_generation(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let hand_before_cast = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    assert_eq!(hand_before_cast, 3);
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, spontaneous_generation());
    let hand_at_resolution = hand_before_cast - 1;
    pass_until(&mut engine, stack_is_empty);

    let tokens = engine
        .state()
        .arena
        .iter()
        .filter(|(_, o)| o.zone == Zone::Battlefield && o.controller == p0 && o.card.is_none())
        .count();
    assert_eq!(
        tokens, hand_at_resolution,
        "Spontaneous Generation creates a 1/1 green Saproling token for each card in hand"
    );
}
