//! `cards/creatures/mv_5/skirge_familiar.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Skirge Familiar — {4}{B} 3/2 with flying — prints one line: "Discard a
/// card: Add {B}." It is a *mana* ability whose whole price is a card and not
/// its own tap symbol, so five Swamps pay the {4}{B} down to an empty pool and
/// every black mana read afterwards came off the Familiar's own ability. The
/// skipped `{T}` is what the untapped permanent shows, and the second
/// activation on the same creature is the other half of the same claim: the
/// price is a card, not the permanent that charges it.
#[test]
#[allow(clippy::too_many_lines)]
fn skirge_familiar_discards_a_card_for_black_mana_and_keeps_itself_untapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[skirge_familiar(), dark_ritual(), silence()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {4}{B} off five Swamps, which make nothing but black: the pool reads
    // five and then exactly nothing, so the mana below has no other source on
    // this board.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        5,
        "five Swamps tapped for five black"
    );
    cast_with_floating(&mut engine, p0, skirge_familiar());
    pass_until(&mut engine, stack_is_empty);

    let familiar = on_battlefield(&engine, p0, skirge_familiar()).expect("the Familiar resolved");
    assert_eq!(pt(&engine, familiar), (3, 2), "the body the card prints");
    assert!(
        keywords(&engine, familiar).contains(KeywordSet::FLYING),
        "the printed flying reaches the permanent"
    );
    assert!(
        !is_tapped(&engine, familiar),
        "and the creature arrives untapped, so its own {{T}} is still there to \
         *not* be spent"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{4}}{{B}} was spent to the last mana"
    );

    let fodder = in_hand(&engine, p0, dark_ritual()).expect("the fodder is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(familiar, 0)),
        "a card in hand pays the only price the Familiar charges: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, skirge_familiar(), 0);
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
        ChoicePrompt::CostDiscard,
        "a cost and not a cleanup, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one card, no more and no fewer");
    assert!(
        options.contains(&fodder),
        "a card in hand is the whole of the answer: {options:?}"
    );
    assert!(
        !options.contains(&familiar),
        "the Familiar is a permanent on the battlefield and not a card in hand: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .expect("the card the question offered pays the cost");

    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "one black, off one discarded card"
    );
    assert_eq!(pool.total(), 1, "and nothing else came with it");
    assert!(
        in_graveyard(&engine, p0, dark_ritual()).is_some(),
        "the discarded card goes to its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, skirge_familiar()).is_some(),
        "and the Familiar outlives it: the price was the card and not the creature"
    );
    assert!(
        !is_tapped(&engine, familiar),
        "nothing in the price is a {{T}}, so the creature is still standing"
    );

    // The same permanent charges the same price again, which is what says a
    // card is the whole cost rather than a one-shot sacrifice spelled as a
    // discard.
    let second = in_hand(&engine, p0, silence()).expect("a second card is still in hand");
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the seat holds priority again, got {:?}",
        engine.pending()
    );
    activate(&mut engine, p0, skirge_familiar(), 0);
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!(
            "the second activation asks the same cost, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&second),
        "the second card is on the same menu: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![second],
            },
        )
        .expect("the card the question offered pays the cost");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        2,
        "one black per discarded card, and the Familiar is still the one making them"
    );
}
