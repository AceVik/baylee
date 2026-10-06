//! `cards/artifacts/mv_3/phyrexian_vault.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Phyrexian Vault — {3} artifact: "{2}, {T}, Sacrifice a creature: Draw a
/// card."
///
/// All three parts of the price land where a test can read them, and each needs
/// a different reading: the {2} out of a pool only the Forests paid into, the
/// tap on the artifact itself, and the creature in its owner's graveyard
/// rather than merely gone. "A creature" is where the menu does the work — the
/// Elf across the table is as much a creature as mine and must not be on it,
/// the Vault is an artifact and so cannot eat itself, and a refused answer
/// costs the other seat nothing. The draw is the half no pool reading can see,
/// so it is asserted as a move off the library and into the hand together.
#[allow(clippy::too_many_lines)] // one activation, every part of its price read off a different zone
#[test]
fn phyrexian_vault_eats_a_creature_of_its_own_side_for_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[phyrexian_vault()])
        // A creature across the table: "a creature" is read as the seat's own,
        // and a same-card bystander is the only thing that can say so.
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the fodder is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");

    // {3} off three of the five Forests, with the Elf named as the printing
    // kept back: it is the creature the ability is about to ask for, and it
    // taps for mana of its own if the helper is not told otherwise.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Forests, and the Elf contributed nothing"
    );
    cast_with_floating(&mut engine, p0, phyrexian_vault());
    pass_until(&mut engine, stack_is_empty);
    let vault = on_battlefield(&engine, p0, phyrexian_vault()).expect("the Vault resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the {{3}} is spent and the {{2}} the ability charges is still in the pool"
    );

    // The offer is read off the pool and not off the untapped lands, so the
    // mana has to be floating before anything is claimed about it.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(vault, 0)),
        "{{2}}, {{T}}, Sacrifice a creature — the only line the card prints, \
         now that its {{2}} is payable: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(&mut engine, p0, phyrexian_vault(), 0);
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
            "the sacrifice is chosen before it is paid: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat answers its own cost");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one creature, no more and no fewer");
    assert_eq!(
        options,
        vec![elf],
        "the one creature this seat controls is the whole menu: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "`CR 701.21a`: an opponent's creature is not yours to sacrifice: {options:?}"
    );
    assert!(
        !options.contains(&vault),
        "the Vault is an artifact and no creature: {options:?}"
    );

    let refused = engine.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![theirs],
        },
    );
    assert!(
        refused.is_err(),
        "an answer the question did not enumerate is refused: {refused:?}"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and the refusal costs the other seat nothing"
    );

    // CR 601.2c named no target here, and CR 601.2h pays the whole cost at
    // once: the {T}, the {2} and the creature all go with this answer.
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the creature the question offered pays the cost");

    assert!(
        is_tapped(&engine, vault),
        "{{T}} is part of the price and is paid as the ability is activated"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "a sacrificed creature goes to its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "and has left the battlefield, which is the sacrifice itself"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}} it charges came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "drawing a card is no mana ability, so the ability uses the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card left the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "and it is in hand, so an emptied library would not satisfy the count above"
    );
    assert!(
        on_battlefield(&engine, p0, phyrexian_vault()).is_some(),
        "the Vault outlives the creature it ate"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and nothing of the opponent's ever moved"
    );
}
