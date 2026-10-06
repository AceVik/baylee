//! `cards/enchantments/mv_2/sylvan_library.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// "At the beginning of your draw step, you may draw two additional cards.
/// If you do, choose two cards in your hand drawn this turn. For each of
/// those cards, pay 4 life or put the card on top of your library." — both
/// kept: eight life for them, and three cards more in hand.
#[test]
fn sylvan_library_keeps_both_cards_for_eight_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(403, forest())
        .battlefield(0, &[sylvan_library()])
        .start();
    keep_mulligans(&mut engine);
    let (chosen, hand_before) = sylvan_library_offer(&mut engine, p0);
    let Pending::ChooseCards {
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected the put-back question, got {:?}", engine.pending())
    };
    assert_eq!(prompt, ChoicePrompt::PutBackOnTop);
    assert_eq!(options, chosen);
    assert_eq!((min, max), (0, 2), "twenty life pays for both");
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        12,
        "four life for each card kept"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 2
    );
}

/// Both put back, in the order named, and no life paid: the last named is
/// the top card, as every put-back in this engine reads it.
#[test]
fn sylvan_library_puts_both_back_on_top_for_nothing() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(403, forest())
        .battlefield(0, &[sylvan_library()])
        .start();
    keep_mulligans(&mut engine);
    let (chosen, hand_before) = sylvan_library_offer(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: chosen.clone(),
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );
    let library = engine.state().zones.list(ZoneLocation::Library(p0));
    assert_eq!(&library[library.len() - 2..], &[chosen[0], chosen[1]]);
}

/// Five life pays for one card and not two (CR 119.4): at least one of the
/// two has to go back, and the question says so.
#[test]
fn sylvan_library_sends_back_what_the_life_total_cannot_pay_for() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(403, forest())
        .battlefield(0, &[sylvan_library()])
        .life(0, 5)
        .start();
    keep_mulligans(&mut engine);
    let (chosen, _) = sylvan_library_offer(&mut engine, p0);
    let Pending::ChooseCards { min, max, .. } = engine.pending().clone() else {
        panic!("expected the put-back question, got {:?}", engine.pending())
    };
    assert_eq!((min, max), (1, 2), "one of the two must go back");
    assert!(
        engine
            .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
            .is_err(),
        "keeping both is not an answer at five life"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 1);
}
