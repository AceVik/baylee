//! `cards/creatures/mv_2/patrol_hound.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Patrol Hound is a `{1}{W}` 2/2 whose entire rules text is a cost with
/// a question in it: "Discard a card: This creature gains first strike
/// until end of turn." Both halves are played — the Hound is cast so that
/// the permanent is a real one, the card for the cost comes from the hand
/// that the engine itself enumerated, and the keyword is only read after
/// resolution through the layers. The Llanowar Elf under the same control is
/// the control: the pump is `Filter::This`, so no second creature may take
/// the keyword along. That the card is already in the graveyard while the
/// ability is still waiting on the stack is CR 601.2h — payment happens
/// before resolution, and the Hound never taps in the process.
#[test]
fn patrol_hound_discards_a_card_to_gain_first_strike() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), llanowar_elves()])
        .hand(0, &[patrol_hound(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, patrol_hound());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let hound = on_battlefield(&engine, p0, patrol_hound()).expect("the Hound resolved");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is out");
    assert_eq!(pt(&engine, hound), (2, 2), "a printed 2/2");
    assert!(
        !keywords(&engine, hound).contains(KeywordSet::FIRST_STRIKE),
        "nothing has been discarded yet"
    );

    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the seat with the Hound holds priority");
    assert!(
        legal.abilities.contains(&(hound, 0)),
        "the price is a card and no mana, so the one line the card prints is \
         offered as long as the hand is not empty: {:?}",
        legal.abilities
    );

    let fodder = in_hand(&engine, p0, forest()).expect("a Forest is in hand");
    activate(&mut engine, p0, patrol_hound(), 0);

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
            "the discard is a cost and asks which card, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat answers its own cost");
    assert_eq!(
        prompt,
        ChoicePrompt::CostDiscard,
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one card, no more and no fewer");
    assert!(
        options.contains(&fodder),
        "a card in hand is the whole of the answer: {options:?}"
    );
    assert!(
        in_hand(&engine, p0, forest()).is_some(),
        "CR 601.2h: costs are paid after the question, so nothing has left the hand yet"
    );

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .expect("the card the question offered pays the cost");

    assert!(
        in_graveyard(&engine, p0, forest()).is_some(),
        "the discarded card left the hand for its owner's graveyard"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1,
        "exactly one card was given up"
    );
    assert!(
        !stack_is_empty(&engine),
        "gaining a keyword is no mana ability, so the ability is on the stack"
    );
    assert!(
        !is_tapped(&engine, hound),
        "its whole price is the card: the Hound never taps"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, hound).contains(KeywordSet::FIRST_STRIKE),
        "\"This creature gains first strike\""
    );
    assert_eq!(
        pt(&engine, hound),
        (2, 2),
        "the pump is +0/+0 and the keyword: the body does not move"
    );
    assert!(
        !keywords(&engine, elf).contains(KeywordSet::FIRST_STRIKE),
        "`Filter::This` reaches the Hound and no other creature"
    );
}
