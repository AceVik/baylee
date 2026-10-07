//! `cards/lands/towns/the_gold_saucer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The Gold Saucer is a Town land printing three lines and only two of them are
/// written: "{T}: Add {C}" and "{3}, {T}, Sacrifice two artifacts: Draw a
/// card" — the coin flip has no vocabulary, so nothing here asks for one. Both
/// written lines are played on one board: the {3} comes out of a pool three
/// Forests really filled, and "sacrifice two artifacts" is read as *two*
/// questions whose menu is the two Sol Rings this seat controls and never the
/// Saucer itself, which is a land. The turn cycle afterwards is the mana half:
/// the untap step stands the Saucer back up and its own tap is the whole price
/// of one colourless, which no Forest on this board could have made.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn the_gold_saucer_eats_two_artifacts_for_a_card_and_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                the_gold_saucer(),
                forest(),
                forest(),
                forest(),
                quiet_artifact(),
                quiet_artifact(),
            ],
        )
        .battlefield(1, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let saucer = on_battlefield(&engine, p0, the_gold_saucer()).expect("the Saucer is out");
    let theirs =
        on_battlefield(&engine, p1, quiet_artifact()).expect("an artifact across the table");
    assert!(!is_tapped(&engine, saucer), "it enters untapped and ready");
    let rings = all_on_battlefield(&engine, p0, quiet_artifact());
    assert_eq!(rings.len(), 2, "two artifacts are what the draw charges");
    let (first, second) = (rings[0], rings[1]);

    // The Saucer prints a mana ability whose whole price is its own {T}, and so
    // do the two Sol Rings, so all three are named as kept back: `tap_all_mana`
    // would otherwise have spent the very activation this test is about (#159).
    let taken = tap_mana_where(&mut engine, p0, |id| {
        id != saucer && id != first && id != second
    });
    assert_eq!(
        taken, 3,
        "the three Forests, and nothing else on this board"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three green floating: the {{3}} the draw charges is payable"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    let offered: Vec<u32> = legal
        .abilities
        .iter()
        .filter(|(source, _)| *source == saucer)
        .map(|(_, index)| *index)
        .collect();
    assert_eq!(
        offered,
        vec![0, 1],
        "the printed mana ability and the two-artifact draw, and no coin flip: \
         a random outcome has no vocabulary to be written in"
    );

    // Ability 0 is the mana ability; ability 1 is the one that draws.
    activate(&mut engine, p0, the_gold_saucer(), 1);

    // "Sacrifice two artifacts" is two parts, and each part is asked where the
    // rules put it (CR 601.2h) — the menu is read on the spot, which is what
    // says the two are different artifacts rather than one answering for both.
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
            "the sacrifice is a cost and is asked before it is paid: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat answers its own cost");
    assert_eq!(
        prompt,
        ChoicePrompt::CostSacrifice,
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one artifact, and the part asks once");
    assert_eq!(
        options.len(),
        2,
        "the two artifacts this seat controls: {options:?}"
    );
    assert!(
        options.contains(&first) && options.contains(&second),
        "both Sol Rings are on the menu: {options:?}"
    );
    assert!(
        !options.contains(&saucer),
        "the Saucer is a land: \"an artifact\" is read, not skipped: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "\"sacrifice two artifacts\" means two of *yours* (CR 701.21a): the one \
         across the table is not on the menu: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![first],
            },
        )
        .expect("the artifact the question offered pays the cost");

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
            "`Sacrifice two artifacts` is two parts, so it asks a second time: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "and the same seat answers both parts");
    assert_eq!(prompt, ChoicePrompt::CostSacrifice);
    assert_eq!((min, max), (1, 1));
    assert_eq!(
        options.len(),
        1,
        "one artifact is left, and the one already given up is not offered a \
         second time: {options:?}"
    );
    assert!(
        options.contains(&second),
        "and the artifact left over is exactly the second one: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![second],
            },
        )
        .expect("the artifact the second part asked for");

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    assert!(
        is_tapped(&engine, saucer),
        "{{T}} is part of the price and is paid as the ability is activated"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{3}} it charges came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "drawing a card is no mana ability, so the ability is waiting on the stack"
    );
    assert!(
        on_battlefield(&engine, p0, the_gold_saucer()).is_some(),
        "the land is the source of the ability and never part of its price"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        all_on_battlefield(&engine, p0, quiet_artifact()).is_empty(),
        "both artifacts the cost named left the battlefield"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        2,
        "and both are in their owner's graveyard, which is where a sacrificed \
         permanent goes"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card left the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "and it reached the hand, so an emptied library would not satisfy the \
         count above"
    );

    // The mana half needs the untap step, because the draw spent the {T}: a
    // whole turn cycle is what stands the Saucer back up, and the empty pool
    // that comes with it is the CR 500.5 control for the single colourless.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, saucer),
        "the untap step stood the Saucer back up"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool emptied with the step that ended"
    );

    activate(&mut engine, p0, the_gold_saucer(), 0);
    assert!(
        !matches!(engine.pending(), Pending::ChooseColor { .. }),
        "`{{C}}` is a fixed colourless and not \"any color\", so nothing is asked \
         on the way: {:?}",
        engine.pending()
    );
    assert!(is_tapped(&engine, saucer), "the Saucer paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        1,
        "\"{{T}}: Add {{C}}\" — one, off the land's own tap"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "and no green: the Forests are untapped on this turn and were never \
         asked for anything, so the colourless has no other source on the board"
    );
    assert_eq!(pool.total(), 1, "one mana, and nothing else came with it");
}
