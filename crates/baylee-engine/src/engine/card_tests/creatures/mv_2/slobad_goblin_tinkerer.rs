//! `cards/creatures/mv_2/slobad_goblin_tinkerer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn slobad_sacrifices_an_artifact_to_make_another_indestructible() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                plains(),
                plains(),
                swamp(),
                swamp(),
                time_sieve(),
                thopter_foundry(),
            ],
        )
        .hand(0, &[slobad_goblin_tinkerer(), vindicate()])
        // An artifact across the table, so "target artifact" is read as the
        // whole battlefield and not quietly narrowed to this seat's board.
        .battlefield(1, &[chromatic_lantern()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {1}{R} out of the six lands, tapped in one pass so the {1}{W}{B} of the
    // Vindicate below comes out of the same open pool (CR 500.5).
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, slobad_goblin_tinkerer());
    pass_until(&mut engine, stack_is_empty);
    let slobad = on_battlefield(&engine, p0, slobad_goblin_tinkerer()).expect("Slobad resolved");
    let fodder = on_battlefield(&engine, p0, time_sieve()).expect("an artifact to give up");
    let target = on_battlefield(&engine, p0, thopter_foundry()).expect("an artifact to protect");
    let theirs =
        on_battlefield(&engine, p1, chromatic_lantern()).expect("an artifact across the table");
    assert!(
        !keywords(&engine, target).contains(KeywordSet::INDESTRUCTIBLE),
        "nothing has granted anything yet"
    );

    activate(&mut engine, p0, slobad_goblin_tinkerer(), 0);
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
            "`target artifact` is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat chooses");
    assert_eq!((min, max), (1, 1), "one artifact, no more and no fewer");
    assert!(player_options.is_empty(), "no player is an artifact");
    assert!(
        options.contains(&fodder) && options.contains(&target),
        "both artifacts this seat controls may be named: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "`target artifact` is any artifact, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&slobad),
        "Slobad is a creature and no artifact: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .expect("the artifact the question offered was chosen");

    // CR 601.2c before CR 601.2h: the target is named while the price is still
    // unpaid, so the fodder is where it was when the question was asked.
    assert!(
        on_battlefield(&engine, p0, time_sieve()).is_some(),
        "the sacrifice is the last step of the activation, not the first"
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
        panic!("the cost asks which artifact, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat answers its own cost");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "a cost and not a search, which is all a client has to tell them apart"
    );
    assert_eq!((min, max), (1, 1), "one artifact, no more and no fewer");
    assert_eq!(
        options.len(),
        2,
        "the two artifacts this seat controls and nothing else: {options:?}"
    );
    assert!(
        options.contains(&fodder) && options.contains(&target),
        "both are artifacts you control, the one just named included: {options:?}"
    );
    assert!(
        !options.contains(&slobad),
        "Slobad is a creature: `an artifact` is read, not skipped: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "CR 701.21a: an opponent's artifact is not yours to sacrifice: {options:?}"
    );

    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![theirs],
                },
            )
            .is_err(),
        "an answer the question did not enumerate is refused"
    );
    assert!(
        on_battlefield(&engine, p1, chromatic_lantern()).is_some(),
        "and the refusal costs the other seat nothing"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .expect("the artifact the question offered pays the cost");
    assert!(
        in_graveyard(&engine, p0, time_sieve()).is_some(),
        "a sacrificed artifact goes to its owner's graveyard"
    );
    assert!(
        !stack_is_empty(&engine),
        "granting a keyword is no mana ability, so the ability is on the stack"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, thopter_foundry()).is_some(),
        "the artifact that was named never moved"
    );
    assert!(
        keywords(&engine, target).contains(KeywordSet::INDESTRUCTIBLE),
        "`target artifact gains indestructible until end of turn`"
    );

    // The keyword read off the layers is a projection; the destroy is what
    // makes it a rule. Vindicate's `destroy` cannot take an indestructible
    // permanent (CR 702.12b), and it is spent either way.
    cast_with_floating(&mut engine, p0, vindicate());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("Vindicate targets a permanent, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "same seat, same main phase — the grant lasts");
    assert!(
        options.contains(&target),
        "the artifact is still a legal target for a destroy effect: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .expect("the permanent the question offered was chosen");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, thopter_foundry()).is_some(),
        "CR 702.12b: a permanent with indestructible can't be destroyed, so it \
         is still standing"
    );
    assert!(
        in_graveyard(&engine, p0, thopter_foundry()).is_none(),
        "and it was not put into the graveyard"
    );
    assert!(
        in_graveyard(&engine, p0, vindicate()).is_some(),
        "the spell resolved and went to the graveyard, so a destroy really \
         happened and was refused"
    );
}
