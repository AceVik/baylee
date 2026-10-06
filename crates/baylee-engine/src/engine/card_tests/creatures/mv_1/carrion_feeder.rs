//! `cards/creatures/mv_1/carrion_feeder.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Carrion Feeder ({B}, a 1/1 Zombie): "This creature can't block. Sacrifice
/// a creature: Put a +1/+1 counter on this creature."
///
/// Both printed sentences, and the first of them is a *face* keyword rather
/// than a static ability — it is printed on this creature about itself, so
/// the reading that can see it is the projected one and that is the reading
/// `combat::can_block` takes. What the bit does in combat is one module
/// over, in `combat_choice_tests`, where the twin beside it is what makes
/// the absence mean the rule. The sacrifice is a *cost* and not a target
/// (CR 701.21a), so which creatures are on the menu is a board reading: the
/// Feeder is a creature you control and sits on its own menu, the Elves
/// beside it pay, and the Elves across the table are neither offered nor
/// accepted. Paying leaves the card in its owner's graveyard and turns the
/// projected 1/1 into the 2/2 the counter prints — the half of the sentence
/// the card file cannot show, since the effect and the cost are the engine's
/// answer rather than the card's.
#[allow(clippy::too_many_lines)] // both halves of "a creature you control", in one game
#[test]
fn carrion_feeder_eats_a_creature_of_yours_for_a_counter_and_may_not_eat_theirs() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(83, swamp())
        .battlefield(0, &[carrion_feeder(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let feeder = on_battlefield(&engine, p0, carrion_feeder()).expect("the Feeder stands");
    let fodder = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves stand");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves stand");
    assert_eq!(pt(&engine, feeder), (1, 1), "the body it prints");
    assert_eq!(
        counters_on(&engine, feeder, CounterKind::P1P1),
        0,
        "and no counter on it yet"
    );
    assert!(
        keywords(&engine, feeder).contains(KeywordSet::CANT_BLOCK),
        "the restriction is printed on the face, so it is on the creature \
         before anything is activated"
    );
    assert!(
        !keywords(&engine, fodder).contains(KeywordSet::CANT_BLOCK),
        "and the Elves this same board seated the same way do not have it, \
         which is what makes the line above a reading of the card rather \
         than of the preset"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(feeder, 0)),
        "a creature to eat is on the table, so the one line the Feeder \
         prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, carrion_feeder(), 0);
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("the cost asks which creature, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat is the one asked");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "the variant is the whole of what tells a client this is a cost being \
         paid and not a search"
    );
    assert_eq!((min, max), (1, 1), "one creature, and the cost asks once");
    assert!(
        options.contains(&fodder),
        "the Elves are a creature you control: {options:?}"
    );
    assert!(
        options.contains(&feeder),
        "and the Feeder is a creature too, so it is on its own menu: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "`CR 701.21a`: a seat sacrifices only what it controls, whatever the \
         filter says: {options:?}"
    );
    assert_eq!(options.len(), 2, "and those two are the whole menu");

    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![theirs],
                },
            )
            .is_err(),
        "an answer the question did not enumerate is refused before anything moves"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and the refusal costs the other seat nothing"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .expect("the creature the question offered pays the cost");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "the sacrificed creature left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "and is in its owner's graveyard, not exiled"
    );
    assert_eq!(
        counters_on(&engine, feeder, CounterKind::P1P1),
        1,
        "the ability put a +1/+1 counter on the Feeder"
    );
    assert_eq!(
        pt(&engine, feeder),
        (2, 2),
        "a printed 1/1 plus one +1/+1 counter, read through the layers"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the opponent's Elves never moved"
    );
}
