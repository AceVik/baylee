//! `cards/creatures/mv_3/dogged_hunter.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Dogged Hunter — {2}{W} 1/1 Human Nomad: "{T}: Destroy target creature
/// token." Neither word of that reads off the card file, so the scenario
/// plays the token into existence first: Thopter Foundry's own activated
/// ability eats a Sol Ring and leaves a Thopter, which is the only creature
/// token in the game and therefore the only thing the Hunter's filter may
/// name. The Llanowar Elves standing beside it are the control — a filter
/// that had lost `Filter::IsToken` would offer the same printed 1/1 — and the
/// {T} is read where CR 601.2h puts it: after the target question, with the
/// token already off the battlefield and the Elves still on it.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn dogged_hunter_destroys_a_creature_token_and_not_the_creature_beside_it() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                dogged_hunter(),
                thopter_foundry(),
                quiet_artifact(),
                quiet_creature(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let hunter = on_battlefield(&engine, p0, dogged_hunter()).expect("the Hunter is seated");
    let elves = on_battlefield(&engine, p0, quiet_creature()).expect("the Elves are standing");
    let ring = on_battlefield(&engine, p0, quiet_artifact()).expect("the Sol Ring is out");
    assert!(
        !is_tapped(&engine, hunter),
        "nothing has paid the Hunter's {{T}} yet"
    );

    // {1} for the Foundry, out of a pool the two Forests actually filled —
    // and a source it did not have to count on, because a Sol Ring tapped for
    // mana is a Sol Ring that can no longer be the sacrifice this token is
    // made of. So the artifact is the printing kept back, and the mana for the
    // Foundry comes off the lands and the Elf beside it.
    tap_all_mana_but(&mut engine, p0, Some(quiet_artifact()));
    assert!(
        engine.state().players[0].mana_pool.total() >= 1,
        "two Forests and one Llanowar Elves is three mana, and the Foundry \
         charges one: {:?}",
        engine.state().players[0].mana_pool
    );
    activate(&mut engine, p0, thopter_foundry(), 0);
    let Pending::ChooseCards {
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the Foundry asks which artifact to eat, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        prompt,
        ChoicePrompt::CostSacrifice,
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one artifact, no more and no fewer");
    assert!(
        options.contains(&ring),
        "the Sol Ring is the fodder this scenario is built around: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ring],
            },
        )
        .expect("the artifact the question offered pays the cost");
    pass_until(&mut engine, stack_is_empty);

    let tokens = tokens_of(&engine, p0);
    assert_eq!(
        tokens.len(),
        1,
        "one activation of the Foundry, one Thopter — the only creature token \
         in the game, and so the only thing the Hunter may name"
    );
    let thopter = tokens[0];
    assert!(
        types(&engine, thopter).contains(TypeSet::CREATURE),
        "an artifact *creature* token, so \"target creature token\" has a target"
    );

    // The offer is read off the board the Hunter stands on; its whole price is
    // its own {T} and it still has it.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(hunter, 0)),
        "the one line the card prints is offered: {:?}",
        legal.abilities
    );
    activate(&mut engine, p0, dogged_hunter(), 0);

    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature token\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat aims it");
    assert_eq!((min, max), (1, 1), "exactly one creature token");
    assert!(
        player_options.is_empty(),
        "the card names no player, so no player is on the menu: {player_options:?}"
    );
    assert!(
        options.contains(&thopter),
        "the token the Foundry made is a creature token: {options:?}"
    );
    assert!(
        !options.contains(&elves),
        "a printed 1/1 that is no token is not a creature token, so it is not \
         on the menu: {options:?}"
    );
    assert_eq!(
        options.len(),
        1,
        "and the token is the whole of it — no artifact, no land, and not the \
         Hunter itself, which is a creature and no token: {options:?}"
    );
    assert!(
        !is_tapped(&engine, hunter),
        "CR 601.2c before CR 601.2h: the target is named before the tap is paid"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![thopter],
            },
        )
        .expect("the token the question offered was chosen");
    assert!(
        is_tapped(&engine, hunter),
        "{{T}} is the whole price, and the last step of the activation is \
         where it is paid (CR 601.2h)"
    );
    assert!(
        !stack_is_empty(&engine),
        "destroying a creature is no mana ability, so it uses the stack"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        tokens_of(&engine, p0).is_empty(),
        "the token left the battlefield: nothing on this side is a token"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_creature()).is_some(),
        "the creature the ability did not name never moved"
    );
    assert!(
        on_battlefield(&engine, p0, dogged_hunter()).is_some(),
        "and the Hunter outlives what it aimed at"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        1,
        "the Sol Ring the Foundry ate is the only card in the graveyard: a \
         destroyed token ceases to exist (CR 704.5d) rather than being buried"
    );
}
