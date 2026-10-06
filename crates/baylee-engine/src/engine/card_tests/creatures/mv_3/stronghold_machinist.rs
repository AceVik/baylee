//! `cards/creatures/mv_3/stronghold_machinist.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Stronghold Machinist — {2}{U}, 1/1 Human Spellshaper: "{U}{U}, {T},
/// Discard a card: Counter target noncreature spell."
///
/// The scenario pays all three parts of that price, each of which leaves a
/// mark somewhere else: the {U}{U} out of a pool two Islands actually filled,
/// the tap symbol off the creature itself, and the discarded card in its
/// owner's graveyard. The spell being countered is a Dark Ritual that is
/// really on the stack — cast for real, so the counter has something to aim
/// at — and the proof it died there is that its controller's pool never
/// grows by the three black it would have made.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn stronghold_machinist_discards_a_card_to_counter_a_noncreature_spell() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), stronghold_machinist()])
        // The price the Spellshaper pays in cards: a printing that stands
        // nowhere else on the board, so the graveyard entry below can only be
        // this discard.
        .hand(0, &[llanowar_elves()])
        .battlefield(1, &[swamp()])
        .hand(1, &[dark_ritual()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    // The spell that is about to be countered, cast for real: an instant with
    // no target of its own (CR 601.2c asks nothing), paid by the one Swamp.
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, dark_ritual());

    // p1's own priority passes to p0 with the Ritual still on the stack, which
    // is the window a counterspell exists for.
    pass_until(&mut engine, |e| {
        on_stack(e, dark_ritual()).is_some()
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    let ritual = on_stack(&engine, dark_ritual()).expect("the Ritual is on the stack");
    let machinist =
        on_battlefield(&engine, p0, stronghold_machinist()).expect("the Machinist is out");

    // Mana before the claim: `legal.abilities` is filtered through
    // `can_afford`, which reads the pool and not the untapped Islands.
    tap_all_mana_but(&mut engine, p0, Some(stronghold_machinist()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        2,
        "two Islands tapped, and the Machinist makes no mana of its own"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(machinist, 0)),
        "the one line the card prints, now that {{U}}{{U}} is in the pool: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, stronghold_machinist(), 0);

    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target noncreature spell\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat aims it");
    assert!(
        options.contains(&ritual),
        "the Ritual is the noncreature spell on the stack: {options:?}"
    );
    // CR 601.2c picks the target and CR 601.2h pays afterwards, so while this
    // question stands nothing at all has been given up.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the {{U}}{{U}} is still floating"
    );
    assert!(
        !is_tapped(&engine, machinist),
        "and the Machinist is still standing"
    );
    assert!(
        in_hand(&engine, p0, llanowar_elves()).is_some(),
        "and the card to discard is still in hand"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ritual],
            },
        )
        .expect("the spell the question offered is a legal target");

    // The card is the last part of the price, and it is asked off the hand.
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
    let fodder = in_hand(&engine, p0, llanowar_elves()).expect("the card to discard is in hand");
    assert!(
        options.contains(&fodder),
        "any card in hand pays a `Discard a card` price: {options:?}"
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
        is_tapped(&engine, machinist),
        "{{T}} was the second half of the price"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{U}}{{U}} the third"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_stack(&engine, dark_ritual()).is_none(),
        "the Ritual left the stack"
    );
    assert!(
        in_graveyard(&engine, p1, dark_ritual()).is_some(),
        "a countered spell goes to its owner's graveyard (CR 701.6a)"
    );
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        0,
        "and it never resolved: the three black it would have made are not there"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "the discarded card is in its owner's graveyard"
    );
}
