//! `cards/creatures/mv_2/waterfront_bouncer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Waterfront Bouncer — {1}{U}, 1/1 Merfolk Spellshaper: "{U}, {T}, Discard a
/// card: Return target creature to its owner's hand."
///
/// The price is three separate things and each one leaves a mark somewhere a
/// test can read it: the blue out of a pool the three Islands actually
/// filled, the {T} on the 1/1 itself, and the discarded card in its owner's
/// graveyard. The order is the rules': the target is named at CR 601.2c and
/// everything is paid at CR 601.2h, so while the target question stands the
/// Bouncer is untapped and the blue is still floating — asserted rather than
/// assumed, because a cost paid on announcement would satisfy every count
/// below just as well. The target is the opponent's Elf, which is what makes
/// "to its *owner's* hand" a claim about which hand it lands in and not only
/// about it leaving the battlefield.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn waterfront_bouncer_pitches_a_card_to_bounce_a_creature_to_its_owners_hand() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), waterfront_bouncer()])
        .hand(0, &[counterspell()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bouncer = on_battlefield(&engine, p0, waterfront_bouncer()).expect("the Bouncer is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    let pitched = in_hand(&engine, p0, counterspell()).expect("a card to pitch is in hand");

    // Mana before the claim: `can_afford` reads the pool and never the
    // untapped Islands, so the offer is only worth reading once the three are
    // floating.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Islands, and the Bouncer makes no mana of its own"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(bouncer, 0)),
        "with the {{U}} floating, the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, waterfront_bouncer(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "`target creature` is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat aims it");
    assert!(
        options.contains(&theirs),
        "\"target creature\" reaches across the table: {options:?}"
    );
    assert!(
        options.contains(&bouncer),
        "and it is any creature, its own source included — the card does not \
         say \"another\": {options:?}"
    );
    assert!(
        !is_tapped(&engine, bouncer),
        "CR 601.2c names the target before CR 601.2h pays for it, so the \
         {{T}} has not happened while the question stands"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "and neither has the {{U}}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the Elf was one of the options it enumerated");

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
            "`Discard a card` is a cost and is asked once a target exists, \
             got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat pays its own price");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostDiscard,
        "a cost and not a cleanup discard, which is all a client has to tell \
         the two apart"
    );
    assert_eq!((min, max), (1, 1), "one card, no more and no fewer");
    assert!(
        options.contains(&pitched),
        "the card named in hand is on the menu: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![pitched],
            },
        )
        .expect("the card the question offered pays the cost");

    assert!(
        is_tapped(&engine, bouncer),
        "{{T}} is the second part of the price"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "and the {{U}} came out of the pool"
    );
    assert!(
        in_graveyard(&engine, p0, counterspell()).is_some(),
        "the discarded card is in its owner's graveyard"
    );
    assert!(
        !stack_is_empty(&engine),
        "returning a creature is no mana ability, so the ability is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "the targeted creature left the battlefield"
    );
    assert!(
        in_hand(&engine, p1, llanowar_elves()).is_some(),
        "\"to its owner's hand\" — the Elf goes back to the seat that owns it"
    );
    assert!(
        in_hand(&engine, p0, llanowar_elves()).is_none(),
        "and not to the seat that aimed the bounce"
    );
    assert!(
        on_battlefield(&engine, p0, waterfront_bouncer()).is_some(),
        "and the Bouncer spent its tap and a card, never itself"
    );
}
