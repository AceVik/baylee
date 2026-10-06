//! `cards/enchantments/mv_3/mental_discipline.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "b22080d6-a9ed-4bdd-a604-058e0e3e9463"

/// Mental Discipline — {1}{U}{U} enchantment: "{1}{U}, Discard a card: Draw a
/// card." The price and the effect each land somewhere a test can read, and one
/// activation reads all four places at once: the {1}{U} comes out of a pool the
/// five Islands filled for the cast, the discarded card is in its owner's
/// graveyard rather than merely gone from the hand, the drawn card is off the
/// top of the library, and the enchantment itself survives the activation. The
/// engine asks which card is being given up as `CostDiscard` — a cost, not a
/// search — and the hand holds exactly the one card that was drawn afterwards.
#[test]
fn mental_discipline_spends_mana_and_a_card_to_draw_a_card() {
    let p0 = PlayerId::new(0);
    let fodder = silence();
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .hand(0, &[mental_discipline(), fodder])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The {1}{U}{U} cast is paid out of the pool: `cast_from_hand` taps all
    // five Islands and leaves exactly the {1}{U} the ability charges floating.
    cast_from_hand(&mut engine, p0, mental_discipline());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, mental_discipline()).is_some(),
        "the enchantment resolved onto the table"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "five Islands less the {{1}}{{U}}{{U}} the enchantment costs"
    );

    let library_before = library_size(&engine, p0);
    let fodder_card = in_hand(&engine, p0, fodder).expect("the fodder is in hand");

    activate(&mut engine, p0, mental_discipline(), 0);
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("the discard is a cost, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat answers its own cost");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostDiscard,
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one card, no more and no fewer");
    assert_eq!(
        options,
        vec![fodder_card],
        "the only card left in hand is the whole of the menu"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder_card],
            },
        )
        .expect("the card the question offered pays the cost");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}}{{U}} it charges came out of the pool"
    );
    assert!(
        in_graveyard(&engine, p0, fodder).is_some(),
        "a discarded card goes to its owner's graveyard, not merely out of the hand"
    );
    assert!(
        !stack_is_empty(&engine),
        "drawing a card is no mana ability, so the effect is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"draw a card\": one card off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        1,
        "one card discarded and one drawn, so the hand holds exactly the new card"
    );
    assert!(
        on_battlefield(&engine, p0, mental_discipline()).is_some(),
        "an activated ability costs the enchantment nothing but the mana it was paid"
    );
}
