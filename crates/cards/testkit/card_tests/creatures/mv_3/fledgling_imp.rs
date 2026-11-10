//! `cards/creatures/mv_3/fledgling_imp.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fledgling Imp is a {2}{B} 2/2 whose whole printed text is one activated
/// line: "{B}, Discard a card: This creature gains flying until end of turn."
///
/// Both halves of that price are read in a different place, and each needs its
/// own control. The {B} is a real payment out of a pool the Swamps actually
/// filled (the pool is read *after* the cast, so the single mana left over is
/// the ability's own), and the discard is asked as a cost — `CostDiscard`, one
/// card, out of the hand — before anything is paid (CR 601.2h), which is why
/// the Imp is still grounded while the question stands. The granted keyword is
/// the half no card file can show: an Elf beside it is the creature "this
/// creature" must *not* reach.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn fledgling_imp_discards_a_card_for_its_own_flying_and_no_one_elses() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(211, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), llanowar_elves()])
        .hand(0, &[fledgling_imp(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Mana into the pool first: "is the creature castable" and "is the ability
    // offered" are both read off the pool and never off the untapped lands.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "four Swamps and the Elf's own {{G}} are every mana source on this board"
    );

    cast_with_floating(&mut engine, p0, fledgling_imp());
    pass_until(&mut engine, stack_is_empty);

    let imp = on_battlefield(&engine, p0, fledgling_imp()).expect("the Imp resolved");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is out");
    assert_eq!(pt(&engine, imp), (2, 2), "the body the card prints");
    assert!(
        !keywords(&engine, imp).contains(KeywordSet::FLYING),
        "a grounded 2/2 until something grants it flying"
    );
    assert!(
        !keywords(&engine, elves).contains(KeywordSet::FLYING),
        "and the Elf beside it is grounded too"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "{{2}}{{B}} left two of the five in the pool"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(imp, 0)),
        "with {{B}} floating the one line the card prints is offered: {:?}",
        legal.abilities
    );
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(&mut engine, p0, fledgling_imp(), 0);
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
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one card, no more and no fewer");
    assert_eq!(
        options.len(),
        hand_before,
        "every card in hand is a legal answer: {options:?}"
    );
    assert!(
        !keywords(&engine, imp).contains(KeywordSet::FLYING),
        "the pump resolves off the stack, so nothing is granted while the cost is still open"
    );

    let discarded = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![discarded],
            },
        )
        .expect("a card the cost question offered is a legal answer");

    assert!(
        engine
            .state()
            .object(discarded)
            .is_some_and(|o| o.zone == Zone::Graveyard),
        "CR 701.9a: a discarded card goes to its owner's graveyard"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1,
        "and it left the hand it was discarded from"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{B}} half of the price went with it"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        keywords(&engine, imp).contains(KeywordSet::FLYING),
        "\"This creature gains flying until end of turn\""
    );
    assert!(
        !keywords(&engine, elves).contains(KeywordSet::FLYING),
        "\"this creature\" is not \"creatures you control\": the Elf beside it stays grounded"
    );
}
