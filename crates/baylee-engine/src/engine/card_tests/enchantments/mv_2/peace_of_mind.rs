//! `cards/enchantments/mv_2/peace_of_mind.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Peace of Mind — {1}{W} enchantment: "{W}, Discard a card: You gain 3
/// life." Three printed things, and each is read off a different place. The
/// {W} is the offer filtered through `can_afford`, which reads the pool and
/// not the untapped Plains — so the line is absent on a bare board and
/// present once the mana floats. The discard arrives as `CostDiscard` over
/// the hand, and the three life is the *effect*, which is why the life total
/// is asserted before the stack drains and again after. The Mountain is the
/// card that is given up so that "one card fewer" is a fact about the hand
/// rather than a count of the filler deck beside it.
#[test]
fn peace_of_mind_discards_a_card_for_white_and_three_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), peace_of_mind()])
        .hand(0, &[mountain()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let enchantment = on_battlefield(&engine, p0, peace_of_mind()).expect("Peace of Mind is out");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(enchantment, 0)),
        "an empty pool pays no {{W}}, and `can_afford` reads the pool: {:?}",
        legal.abilities
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1,
        "the one Plains pays the one white the ability charges"
    );

    let fodder = in_hand(&engine, p0, mountain()).expect("the Mountain is in hand");
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let life_before = engine.state().players[0].life;

    activate(&mut engine, p0, peace_of_mind(), 0);
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the discard is a cost and is asked, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat gives the card up");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostDiscard,
        "a cost and not an effect, which is all a client has to tell them apart"
    );
    assert_eq!((min, max), (1, 1), "one card, no more and no fewer");
    assert!(
        options.contains(&fodder),
        "\"discard a card\" is any card in the hand: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .expect("a card the question offered pays the cost");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{W}} went with it"
    );
    assert!(
        !stack_is_empty(&engine),
        "gaining life is no mana ability, so the ability is on the stack"
    );
    assert_eq!(
        engine.state().players[0].life,
        life_before,
        "the price is paid on announcement; the life is what resolution is for"
    );
    assert!(
        in_graveyard(&engine, p0, mountain()).is_some(),
        "the discarded card is in its owner's graveyard"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1,
        "and it left the hand behind, which a library that shrank could not say"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        life_before + 3,
        "\"You gain 3 life\" — three, and not one per card in any graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, peace_of_mind()).is_some(),
        "an activated ability costs the enchantment nothing but the card it ate"
    );
}
