//! `cards/creatures/mv_2/plague_witch.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Plague Witch — {1}{B}, 1/1 Elf Spellshaper: "{B}, {T}, Discard a card:
/// Target creature gets -1/-1 until end of turn."
///
/// Three parts of one price are each read somewhere different: the {B} comes
/// out of a pool only the Swamp filled, the {T} leaves the Witch tapped, and
/// the discarded card leaves the hand for the graveyard — read *before* the
/// target is named (CR 601.2c, then 601.2h), because while the target question
/// stands the Witch is still untapped and the card is still in hand. The
/// control is the Elf across the table: "target creature" offers it, so the
/// -1/-1 is proven to land on the creature that was named and not on a board
/// sweep. The second Sol Ring in hand is the discard, and the Witch's own
/// p/t is read after resolution to show the pump is aimed outward.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn plague_witch_discards_a_card_and_a_swamp_to_shrink_the_creature_it_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), plague_witch()])
        .hand(0, &[quiet_artifact()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let witch = on_battlefield(&engine, p0, plague_witch()).expect("the Witch is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    let fodder = in_hand(&engine, p0, quiet_artifact()).expect("the Sol Ring is in hand");
    assert_eq!(pt(&engine, witch), (1, 1), "a printed 1/1");
    assert_eq!(pt(&engine, theirs), (1, 1), "and a 1/1 to shrink");

    // The one Swamp is the whole pool, and the offer is read off it.
    tap_mana_except(&mut engine, p0, witch);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "one Swamp tapped for the {{B}} the ability charges"
    );
    assert!(
        !is_tapped(&engine, witch),
        "and the Witch is still standing"
    );

    activate(&mut engine, p0, plague_witch(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat aims it");
    assert!(
        options.contains(&theirs),
        "the Elf across the table is a creature and so a legal target: {options:?}"
    );

    // CR 601.2c before CR 601.2h: nothing is paid while the question stands.
    assert!(
        !is_tapped(&engine, witch),
        "the {{T}} is the last step of the activation, so the Witch is still untapped"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "and the {{B}} is still in the pool"
    );
    assert!(
        in_hand(&engine, p0, quiet_artifact()).is_some(),
        "and the discard has not happened either"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the Elf was one of the options it enumerated");

    // Now the costs: {B}, {T}, and a card. The discard asks its own question
    // before the ability resolves.
    if let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    {
        assert_eq!(player, p0, "the activating seat gives up its own card");
        assert_eq!(
            prompt,
            crate::choice::ChoicePrompt::CostDiscard,
            "a cost and not a search, which is all a client has to tell them apart"
        );
        assert_eq!((min, max), (1, 1), "one card, and the cost asks once");
        assert_eq!(
            options,
            vec![fodder],
            "the Sol Ring is the only card in hand, so it is the whole menu"
        );
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![fodder],
                },
            )
            .expect("the card the question offered pays the cost");
    } else {
        panic!(
            "`Discard a card` is a cost and is asked, got {:?}",
            engine.pending()
        );
    }

    assert!(
        is_tapped(&engine, witch),
        "{{T}} is paid with the rest of the cost"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{B}} came out of the pool"
    );
    assert!(
        in_graveyard(&engine, p0, quiet_artifact()).is_some(),
        "the discarded card is in its owner's graveyard"
    );
    assert!(!stack_is_empty(&engine), "shrinking is no mana ability");
    pass_until(&mut engine, stack_is_empty);

    // Not a 0/0 on the battlefield: CR 704.5a puts a creature with toughness
    // zero into its owner's graveyard before anybody gets priority back, and
    // a card in a graveyard projects its printed self again — so reading
    // `pt` here would read the 1/1 the printing has and call the ability a
    // failure. Where it went is the assertion.
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "-1/-1 on a printed 1/1 leaves nothing behind"
    );
    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&theirs),
        "and the battlefield is where it is not"
    );
    assert_eq!(
        pt(&engine, witch),
        (1, 1),
        "and the Witch's own body is untouched: the pump is aimed outward"
    );
}
