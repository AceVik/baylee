//! `cards/creatures/mv_3/blaster_mage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Blaster Mage — {2}{R}, 2/2 Human Spellshaper: "{R}, {T}, Discard a card:
/// Destroy target Wall."
///
/// The three parts of the price each leave a mark somewhere a test can read —
/// the {R} empties the single Mountain that paid it, the {T} taps the Mage, and
/// the discarded card lands in its owner's graveyard — so one activation says
/// which of them the engine actually charged. The target filter is read off the
/// offer rather than off the card file: two Walls stand on the table, one of
/// them across it, and the Llanowar Elf beside them together with the Mage
/// itself are the creatures a bare "target creature" would have named. Only the
/// Wall that was pointed at dies, which is the control that the other one was
/// not merely missed by a wipe.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn blaster_mage_discards_and_taps_to_kill_the_wall_it_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[blaster_mage(), mountain(), wall_of_roots()])
        .battlefield(1, &[wall_of_roots(), llanowar_elves()])
        .hand(0, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mage = on_battlefield(&engine, p0, blaster_mage()).expect("the Mage is on the table");
    let mine = on_battlefield(&engine, p0, wall_of_roots()).expect("my Wall is out");
    let theirs = on_battlefield(&engine, p1, wall_of_roots()).expect("their Wall is out");
    let elves = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    let fodder = in_hand(&engine, p0, forest()).expect("the card to discard is in hand");

    // The whole price is a printed {R} and its own {T}, so the Mountain is the
    // only route on this board: the Wall's mana ability pays a -0/-1 counter
    // and the Mage's price is not its tap, so neither is one (#159).
    let taken = tap_all_mana(&mut engine, p0);
    assert_eq!(
        taken, 1,
        "one Mountain, and neither the Wall nor the Mage is a mana route"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one red, which is exactly the {{R}} the ability charges"
    );

    activate(&mut engine, p0, blaster_mage(), 0);

    // CR 601.2c names the target and CR 601.2h pays everything else, so the two
    // questions are answered where they arrive rather than where they were
    // expected, and both halves of the price are read while the other question
    // is still open.
    let mut aimed = false;
    let mut discarded = false;
    for _ in 0..12 {
        if aimed && discarded {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player, options, ..
            } => {
                assert_eq!(player, p0, "the activating seat aims it");
                assert!(
                    options.contains(&mine) && options.contains(&theirs),
                    "\"target Wall\" is any Wall, on either side of the table: {options:?}"
                );
                assert!(
                    !options.contains(&elves),
                    "a Llanowar Elf is no Wall: {options:?}"
                );
                assert!(
                    !options.contains(&mage),
                    "a Human Spellshaper is no Wall either: {options:?}"
                );
                assert_eq!(
                    options.len(),
                    2,
                    "the two Walls are the whole menu: {options:?}"
                );
                assert!(
                    !is_tapped(&engine, mage),
                    "CR 601.2h pays last: the tap has not happened while the \
                     target is still being chosen"
                );
                assert_eq!(
                    engine.state().players[0].mana_pool.total(),
                    1,
                    "and neither has the {{R}}"
                );
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![theirs],
                        },
                    )
                    .expect("the Wall across the table was one of the options");
                aimed = true;
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                prompt,
                ..
            } => {
                assert_eq!(player, p0, "the activating seat gives up its own card");
                assert_eq!(
                    prompt,
                    crate::choice::ChoicePrompt::CostDiscard,
                    "a cost and not a cleanup, which is all a client has to tell \
                     the two apart"
                );
                assert_eq!((min, max), (1, 1), "one card, no more and no fewer");
                assert!(
                    options.contains(&fodder),
                    "the card in hand is the whole of the answer: {options:?}"
                );
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![fodder],
                        },
                    )
                    .expect("the card the question offered pays the cost");
                discarded = true;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while the Mage's ability is paid for: {other:?}"),
        }
    }
    assert!(
        aimed && discarded,
        "both halves of the activation were asked"
    );

    assert!(
        !stack_is_empty(&engine),
        "destroying a Wall is no mana ability, so the effect is on the stack"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, wall_of_roots()).is_some(),
        "the Wall that was named is destroyed, and it goes to its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, wall_of_roots()).is_none(),
        "and it left the battlefield"
    );
    assert!(
        on_battlefield(&engine, p0, wall_of_roots()).is_some(),
        "the other Wall was not the target and is still standing"
    );
    assert!(
        in_graveyard(&engine, p0, forest()).is_some(),
        "\"Discard a card\" put the card in the graveyard of the seat that gave it up"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the Elf was never a legal target and never moved"
    );
    assert!(is_tapped(&engine, mage), "{{T}} was paid");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{R}} came out of the pool"
    );
}
