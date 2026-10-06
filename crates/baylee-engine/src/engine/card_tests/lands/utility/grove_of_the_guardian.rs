//! `cards/lands/utility/grove_of_the_guardian.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Grove of the Guardian prints one small ability and one enormous one:
/// "{T}: Add {C}" and "{3}{G}{W}, {T}, Tap two untapped creatures you control,
/// Sacrifice this land: Create an 8/8 green and white Elemental creature token
/// with vigilance."
///
/// All four parts of that price are paid in one activation, which is what makes
/// the scenario worth playing: five lands' worth of mana leaves the pool, the
/// land taps itself, two *separate* creatures each answer their own tap
/// question — the second menu holding only the creature the first did not take
/// (CR 118.3) — and the land is sacrificed, so the 8/8 is all that is left of
/// it. The creature across the table is the control for "you control", and
/// reading the offer before the mana is tapped is the control for the price:
/// an empty pool pays no {3}{G}{W}, and a line the pool cannot pay is not
/// offered at all.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn grove_of_the_guardian_taps_two_creatures_and_itself_for_an_eight_eight_elemental() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                plains(),
                plains(),
                myr_retriever(),
                myr_retriever(),
            ],
        )
        .hand(0, &[grove_of_the_guardian()])
        // A creature across the table: "you control" is the word that keeps it
        // off the tap menu, and the same card on both sides is the only
        // witness that can say so.
        .battlefield(1, &[myr_retriever()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The land drop is played for real, so the permanent under test arrived
    // the way a land arrives.
    let grove = play_land(&mut engine, p0, grove_of_the_guardian());
    assert!(!is_tapped(&engine, grove), "a land enters untapped");

    let mine = all_on_battlefield(&engine, p0, myr_retriever());
    assert_eq!(mine.len(), 2, "two creatures to tap, one each");
    let (first, second) = (mine[0], mine[1]);
    let theirs = on_battlefield(&engine, p1, myr_retriever()).expect("their creature is out");
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "and nothing has been made yet"
    );

    // `LegalActions::abilities` is filtered through `can_afford`, and that
    // reads the *pool* rather than the untapped lands: with nothing floating
    // the {3}{G}{W} is unpayable and the token line is off the offer. Ability
    // 0 is the printed {T}: Add {C}; ability 1 is the one this test plays.
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the seat with the Grove holds it");
    assert!(
        !legal.abilities.contains(&(grove, 1)),
        "an empty pool pays no {{3}}{{G}}{{W}}, so the token line is not \
         offered: {:?}",
        legal.abilities
    );

    // Mana before the claim. The Grove is named as the printing kept back,
    // because its own `{T}: Add {C}` costs exactly its own tap — the tap the
    // ability still needs (#17). The two creatures print no mana ability at
    // all, so they are still standing to pay their half of the price.
    tap_all_mana_but(&mut engine, p0, Some(grove_of_the_guardian()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "three Forests and two Plains: exactly the {{3}}{{G}}{{W}}"
    );
    assert!(
        !is_tapped(&engine, grove),
        "the Grove was the one kept back"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(grove, 1)),
        "with all five mana in the pool the whole price is payable: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, grove_of_the_guardian(), 1);

    // "Tap two untapped creatures you control" is two cost parts, so it is two
    // questions of one creature each (CR 601.2h).
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
            "the first tap cost asks which creature, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat answers its own cost");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostTap,
        "a cost and not an effect, which is all a client has to tell the two apart"
    );
    assert_eq!(
        (min, max),
        (1, 1),
        "one creature per part, and the cost asks twice"
    );
    assert!(
        options.contains(&first) && options.contains(&second),
        "both untapped creatures you control are on the menu: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "\"you control\": the creature across the table is not yours to tap: {options:?}"
    );
    assert_eq!(options.len(), 2, "and those two are the whole menu");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![first],
            },
        )
        .expect("the creature the question offered pays the cost");
    assert!(
        !is_tapped(&engine, first),
        "nothing is paid before the last question is answered: CR 601.2h pays \
         the total cost at once"
    );

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("the second tap cost asks again, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the same seat answers the same cost");
    assert_eq!(prompt, crate::choice::ChoicePrompt::CostTap);
    assert_eq!(
        (min, max),
        (1, 1),
        "one creature, and the second part is its own question"
    );
    assert_eq!(
        options,
        vec![second],
        "the first answer comes off the second menu, so one creature cannot \
         pay both taps (CR 118.3)"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![second],
            },
        )
        .expect("the remaining creature pays the second part");
    assert!(
        is_tapped(&engine, first) && is_tapped(&engine, second),
        "both creatures paid a tap each, once the last question was answered"
    );

    // The other halves of the price landed where they can be read: the pool is
    // empty, the land that was tapped and sacrificed is gone, and the ability
    // is waiting on the stack.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{3}}{{G}}{{W}} came out of the pool"
    );
    assert!(
        on_battlefield(&engine, p0, grove_of_the_guardian()).is_none(),
        "\"Sacrifice this land\" is the last part of the cost"
    );
    assert!(
        in_graveyard(&engine, p0, grove_of_the_guardian()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
    assert!(
        !stack_is_empty(&engine),
        "making a token is no mana ability, so the ability is waiting on the stack"
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "and the Elemental arrives on resolution, not on announcement"
    );

    pass_until(&mut engine, stack_is_empty);

    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one activation, one Elemental");
    let elemental = tokens[0];
    let kinds = types(&engine, elemental);
    assert!(
        kinds.contains(TypeSet::CREATURE),
        "the token is a creature: {kinds:?}"
    );
    assert_eq!(
        pt(&engine, elemental),
        (8, 8),
        "the 8/8 body the card prints"
    );
    let printed = engine
        .state()
        .object(elemental)
        .expect("the Elemental is on the battlefield")
        .token
        .expect("it knows which token it is");
    assert!(
        printed.colors.contains(baylee_core::color::Color::Green)
            && printed.colors.contains(baylee_core::color::Color::White),
        "green and white, and not a colourless 8/8"
    );
    assert!(
        printed.keywords.contains(KeywordSet::VIGILANCE),
        "\"with vigilance\", read off the token the card names"
    );

    // The creatures were the price and not the victim: both are still on the
    // battlefield, and the one across the table never moved at all.
    assert_eq!(
        all_on_battlefield(&engine, p0, myr_retriever()).len(),
        2,
        "tapping a creature to pay a cost does not remove it"
    );
    assert!(
        !is_tapped(&engine, theirs),
        "and nothing of the opponent's ever moved"
    );
}
