//! `cards/lands/deserts/hostile_desert.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Hostile Desert — a Desert land printing "{T}: Add {C}" and "{2}, Exile a
/// land card from your graveyard: this land becomes a 3/4 Elemental creature
/// until end of turn, and it is still a land." The scenario proves the second
/// ability is a transformation of the *same* battlefield object rather than a
/// replacement: after it resolves that object reads as both a land and a
/// creature carrying the printed 3/4 body, and both halves of its price really
/// happened — two mana out of the pool and the named land card gone from the
/// graveyard. The Forests pay for it and the Desert is the printing kept back,
/// so the permanent that animates is the one that was never tapped.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn hostile_desert_exiles_a_land_from_the_graveyard_to_become_a_three_four_elemental() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                hostile_desert(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // A creature card in the same graveyard, which "exile a *land* card"
    // has to leave off the menu — without it the menu below would be the
    // whole graveyard and would say nothing about the filter.
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("an Elf on the table");
    crate::sba::destroy(
        engine
            .dev_state_mut(p0)
            .expect("the test kit grants dev commands"),
        elf,
    );
    let buried_elf = in_graveyard(&engine, p0, llanowar_elves()).expect("the Elf died");

    let desert = on_battlefield(&engine, p0, hostile_desert()).expect("the Desert is seated");
    let before = types(&engine, desert);
    assert!(
        before.contains(TypeSet::LAND) && !before.contains(TypeSet::CREATURE),
        "a Desert that has made nothing of itself is a land and nothing else: {before:?}"
    );
    assert!(
        engine
            .state()
            .object(desert)
            .expect("the Desert is an object")
            .characteristics()
            .power
            .is_none(),
        "and it carries no body for the animation below to be given credit for"
    );

    // The land the ability costs: `seed_graveyard` moves the top of the
    // 60-card backing deck — all Forests — into the graveyard, beside the
    // Elf.
    seed_graveyard(&mut engine, p0, 1);
    let fodder = in_graveyard(&engine, p0, forest()).expect("a Forest was milled");

    // Three Forests, one more than the {2} the ability charges. The Desert is
    // named as the printing kept back, because it prints its own "{T}: Add
    // {C}" whose whole price is its own tap (#159) and it is the permanent
    // this test animates.
    tap_all_mana_but(&mut engine, p0, Some(hostile_desert()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three tapped Forests, and the Desert paid nothing"
    );

    // Ability 0 is "{T}: Add {C}", so the transformation is index 1; the offer
    // is read with the mana already floating, which is where `can_afford`
    // reads it.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(desert, 1)),
        "with {{2}} floating and a land to exile the line is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, hostile_desert(), 1);

    // Exiling the land is a cost, so it is asked before it is paid
    // (CR 601.2h), and the menu is the whole of what the filter admits.
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
            "the exile is a cost and is asked before it is paid: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat answers its own cost");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostExile,
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one card, no more and no fewer");
    assert_eq!(
        options,
        vec![fodder],
        "the one land card in this seat's graveyard is the whole menu"
    );
    assert!(
        !options.contains(&buried_elf),
        "and the creature card lying beside it is not a land card"
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
        !stack_is_empty(&engine),
        "the animation is no mana ability, so it is waiting on the stack"
    );
    let unchanged = types(&engine, desert);
    assert!(
        unchanged.contains(TypeSet::LAND) && !unchanged.contains(TypeSet::CREATURE),
        "and nothing has changed yet: the effect is the resolution, not the cost: {unchanged:?}"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{2}} came out of the three the Forests made"
    );
    assert!(
        in_graveyard(&engine, p0, forest()).is_none(),
        "the land the cost named left the graveyard"
    );

    // The same object, read again: a land *and* a creature, with the body the
    // card prints — the whole of what "it's still a land" (CR 205.1b) means.
    let after = types(&engine, desert);
    assert!(
        after.contains(TypeSet::LAND) && after.contains(TypeSet::CREATURE),
        "it became a creature and is still a land: {after:?}"
    );
    assert_eq!(
        pt(&engine, desert),
        (3, 4),
        "the body the ability prints, on a permanent that had none"
    );

    // "until end of turn": a turn later the Desert is a plain land again, so
    // the animation was a duration and not a permanent type change.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        on_battlefield(&engine, p0, hostile_desert()).is_some(),
        "the Desert outlives the turn it animated in"
    );
    let later = types(&engine, desert);
    assert!(
        later.contains(TypeSet::LAND) && !later.contains(TypeSet::CREATURE),
        "the grant lasted the turn it was made in and no longer: {later:?}"
    );
}
