//! `cards/creatures/mv_3/overeager_apprentice.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Overeager Apprentice is a `{2}{B}` 1/2 whose whole printed text is one
/// mana ability: "Discard a card, Sacrifice this creature: Add {B}{B}{B}."
/// Both halves of that price have to actually happen and neither is readable
/// off the card file, so the board is three Swamps and an opening hand of
/// filler: the Swamps pay the cast and leave an empty pool, which makes the
/// three black that appear afterwards something only the Apprentice's own
/// line can have produced. The discard arrives as `ChoicePrompt::CostDiscard`
/// with the cards in hand as its menu — a cost and not a search — and while
/// that question still stands the creature is on the battlefield and the mana
/// is unspent (CR 601.2c before CR 601.2h). Nothing touches the stack
/// (CR 605.3b), and the creature ends up in its owner's graveyard beside the
/// card it discarded.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn overeager_apprentice_discards_a_card_and_eats_itself_for_three_black() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[overeager_apprentice(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {2}{B} off the three Swamps, which spends them down to nothing: every
    // claim about the pool below is read against an empty one.
    cast_from_hand(&mut engine, p0, overeager_apprentice());
    pass_until(&mut engine, stack_is_empty);
    let apprentice = on_battlefield(&engine, p0, overeager_apprentice())
        .expect("the Apprentice resolved onto the table");
    assert_eq!(pt(&engine, apprentice), (1, 2), "the body the card prints");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the three Swamps paid the cast and are spent"
    );

    let hand = engine.state().zones.list(ZoneLocation::Hand(p0)).clone();
    assert!(
        !hand.is_empty(),
        "the opening hand's filler cards are what the discard costs"
    );

    // A printed mana ability is an ordinary `(source, index)` entry in the
    // offer, and this one's price is a discard and the source itself — no
    // mana and no tap, so the offer does not turn on the pool.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(apprentice, 0)),
        "the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, overeager_apprentice(), 0);
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
            "the discard is a cost and is asked before it is paid: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat answers its own cost");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostDiscard,
        "a cost and not a search, which is all a client has to tell them apart"
    );
    assert_eq!((min, max), (1, 1), "one card, no more and no fewer");
    assert!(!options.is_empty(), "there is a card in hand to give up");
    for id in &options {
        assert!(
            hand.contains(id),
            "the menu is the cards in hand: {id:?} is not one of them"
        );
    }
    assert!(
        on_battlefield(&engine, p0, overeager_apprentice()).is_some(),
        "CR 601.2h: the costs are paid after the question, so nothing has moved yet"
    );

    let discarded = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![discarded],
            },
        )
        .expect("a card the question offered pays the discard");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 3, "Add {{B}}{{B}}{{B}}");
    assert_eq!(
        pool.total(),
        3,
        "three black and nothing else, off a pool that was empty a moment ago"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana and the \
         sacrifice are already here"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "and the seat holds priority again, got {:?}",
        engine.pending()
    );

    assert!(
        on_battlefield(&engine, p0, overeager_apprentice()).is_none(),
        "sacrificing the creature is the other half of the price"
    );
    assert!(
        in_graveyard(&engine, p0, overeager_apprentice()).is_some(),
        "a sacrificed permanent goes to its owner's graveyard"
    );
    assert!(
        in_graveyard(&engine, p0, swamp()).is_some(),
        "and the discarded card is in the same graveyard"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand.len() - 1,
        "the hand is one card smaller: the Apprentice left it for the stack \
         and the discard cost one more"
    );
}
