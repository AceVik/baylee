//! `cards/creatures/mv_3/merchant_of_secrets.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Merchant of Secrets — {2}{U} — Creature — Human Wizard (1/1): "When this
/// creature enters, draw a card."
///
/// The draw is read as a *move* and not as a count: the card that was on top
/// of the library before the cast is the one in hand afterwards, and the hand
/// is exactly the size it was — the Merchant leaving it and one card arriving
/// — so a trigger that drew nothing, or drew twice, cannot satisfy both
/// readings at once. Three Islands pay the {2}{U} down to an empty pool, and
/// the body that lands is the printed 1/1, so what is being looked at is a
/// cast creature and its own enters-trigger rather than a placement.
#[test]
fn merchant_of_secrets_draws_one_card_when_it_enters() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[merchant_of_secrets()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The top of the library is its last entry, named before anything is
    // cast, so the draw can be followed to a particular card.
    let library_before = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let top = *library_before.last().expect("p0 has a library");
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    cast_from_hand(&mut engine, p0, merchant_of_secrets());
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "three Islands pay {{2}}{{U}} and nothing is left floating"
    );
    pass_until(&mut engine, stack_is_empty);

    let merchant = on_battlefield(&engine, p0, merchant_of_secrets()).expect("it resolved");
    assert_eq!(pt(&engine, merchant), (1, 1), "the body the card prints");
    assert_eq!(
        library_size(&engine, p0),
        library_before.len() - 1,
        "\"draw a card\" — one card off the top of the library"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&top),
        "and it is the very card that was on top, not merely some card"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "the Merchant left the hand and one card arrived, which is what \
         exactly one draw off one cast puts back"
    );
}
