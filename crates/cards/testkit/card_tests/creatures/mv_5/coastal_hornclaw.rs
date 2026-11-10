//! `cards/creatures/mv_5/coastal_hornclaw.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Coastal Hornclaw is a {4}{U} 3/3 Bird whose whole text is one activated
/// ability: "Sacrifice a land: This creature gains flying until end of turn."
/// The price names no particular land, so the engine has to ask which one, and
/// that menu is half the card: both Forests this seat controls are on it while
/// the Elf beside them is a creature and the Forest across the table is not
/// this seat's to give up (CR 701.21a). The effect is the other half, and its
/// printed duration is read by walking a whole turn cycle — a permanent grant
/// would still be on the Bird afterwards. The price is not the Bird's own {T},
/// which is why the Bird is still untapped once the land is gone.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn coastal_hornclaw_trades_a_land_of_its_own_for_flying_until_the_turn_ends() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                island(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[coastal_hornclaw()])
        // A land across the table: "Sacrifice a land" is no invitation to give
        // up somebody else's (CR 701.21a).
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {4}{U} out of the five Islands, with the two Forests named as the
    // printing kept back: they are this test's sacrifice fodder, and a pool
    // reading is only worth anything when the board can offer the land the
    // ability is about.
    tap_all_mana_but(&mut engine, p0, Some(forest()));
    cast_with_floating(&mut engine, p0, coastal_hornclaw());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, coastal_hornclaw()).is_some()
    });

    let bird = on_battlefield(&engine, p0, coastal_hornclaw()).expect("the Hornclaw resolved");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is out");
    let theirs = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    let lands = all_on_battlefield(&engine, p0, forest());
    assert_eq!(lands.len(), 2, "two of this seat's lands are Forests");
    assert_eq!(pt(&engine, bird), (3, 3), "the body the card prints");
    assert!(
        !keywords(&engine, bird).contains(KeywordSet::FLYING),
        "a printed 3/3 Bird with no keyword of its own, on the ground"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.iter().any(|(source, _)| *source == bird),
        "the one line the card prints is offered while a land is on the \
         board: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, coastal_hornclaw(), 0);
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
            "the sacrifice is chosen before it is paid, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat answers its own cost");
    assert_eq!(
        prompt,
        ChoicePrompt::CostSacrifice,
        "a cost and not a search, which is all a client has to tell them apart"
    );
    assert_eq!((min, max), (1, 1), "one land, no more and no fewer");
    assert!(
        options.contains(&lands[0]) && options.contains(&lands[1]),
        "both lands you control are on the menu: {options:?}"
    );
    assert_eq!(
        options.len(),
        7,
        "the five tapped Islands and the two Forests, and nothing else this \
         seat has: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "the Elf is a creature: \"a land\" is read, not skipped: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "a seat sacrifices only what it controls, whatever the filter says: \
         {options:?}"
    );
    assert!(
        !options.contains(&bird),
        "the Hornclaw is no land, and a permanent cannot be its own price: \
         {options:?}"
    );
    assert!(
        !keywords(&engine, bird).contains(KeywordSet::FLYING),
        "the effect is the resolution, not the cost: nothing has been granted \
         while the question still stands"
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
        on_battlefield(&engine, p1, forest()).is_some(),
        "and the refusal costs the other seat nothing"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![lands[0]],
            },
        )
        .expect("the land the question offered pays the cost");

    assert!(
        in_graveyard(&engine, p0, forest()).is_some(),
        "a sacrificed land goes to its owner's graveyard"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, forest()).len(),
        1,
        "exactly one land was given up, and the other is still standing"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the price was a land and no mana: the one mana the cast left over is \
         still floating"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        keywords(&engine, bird).contains(KeywordSet::FLYING),
        "\"This creature gains flying until end of turn\""
    );
    assert!(
        !is_tapped(&engine, bird),
        "the Bird paid no {{T}} of its own, so it is still standing — a card \
         whose price was its own tap would read the same keyword"
    );
    assert_eq!(
        pt(&engine, bird),
        (3, 3),
        "and the granted keyword is all that was added: the body is untouched"
    );

    // "until end of turn": a whole turn cycle later the Bird is on the ground
    // again — and it is still there, so the keyword left rather than the
    // creature.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        on_battlefield(&engine, p0, coastal_hornclaw()).is_some(),
        "the Hornclaw survived the turn it flew in"
    );
    assert!(
        !keywords(&engine, bird).contains(KeywordSet::FLYING),
        "the grant lasted the turn it was made in and no longer"
    );
}
