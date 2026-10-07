//! `cards/creatures/mv_3/krark_clan_stoker.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Krark-Clan Stoker is `{2}{R}` for a 2/2 Goblin Shaman whose whole text is
/// `{T}, Sacrifice an artifact: Add {R}{R}` — a *mana* ability (CR 605.1)
/// whose price is a tap and an artifact and no mana, so the pool read empty
/// before the activation and exactly two red after it is a statement about
/// the Stoker and about nothing else on the board. The menu the cost opens is
/// the other half: the two Sol Rings under the same seat are on it, while the
/// Elf beside them and the Sol Ring across the table are not, which is
/// `YOUR_ARTIFACT` read rather than skipped (CR 701.21a). And because the
/// Stoker is a creature, it cannot be one of the artifacts it eats — the half
/// a filter widened to "any permanent you control" would quietly lose.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn krark_clan_stoker_taps_and_eats_an_artifact_for_two_red() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                krark_clan_stoker(),
                quiet_artifact(),
                quiet_artifact(),
                llanowar_elves(),
            ],
        )
        // The same artifact on the other side of the table: "sacrifice an
        // artifact" is not an invitation to eat somebody else's.
        .battlefield(1, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let stoker = on_battlefield(&engine, p0, krark_clan_stoker()).expect("the Stoker is out");
    let rings = all_on_battlefield(&engine, p0, quiet_artifact());
    assert_eq!(rings.len(), 2, "two artifacts to choose between");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is out");
    let theirs = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");

    // The price is `{T}` and an artifact and nothing else, so no source has to
    // be tapped before the offer is read — and an empty pool is what makes the
    // two red below exact rather than two red on top of whatever was floating.
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert_eq!(player, p0, "and it is the seat with the Stoker");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats before the Stoker is paid"
    );
    assert!(
        legal.abilities.contains(&(stoker, 0)),
        "an untapped Stoker with an artifact to eat, so its one line is \
         offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, krark_clan_stoker(), 0);
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
        options.contains(&rings[0]) && options.contains(&rings[1]),
        "both Sol Rings are artifacts you control: {options:?}"
    );
    assert!(
        !options.contains(&elves),
        "the Elf is a creature: \"an artifact\" is read, not skipped: {options:?}"
    );
    assert!(
        !options.contains(&stoker),
        "and the Stoker is the other creature on this board — a creature \
         cannot pay an artifact price, not even with itself: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "`CR 701.21a`: an opponent's artifact is not yours to sacrifice: {options:?}"
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
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "and the refusal costs the other seat nothing"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![rings[1]],
            },
        )
        .expect("the artifact the question offered pays the cost");

    assert!(
        stack_is_empty(&engine),
        "`CR 605.3b`: a mana ability uses no stack, so the mana is already here"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "and the seat holds priority again, got {:?}",
        engine.pending()
    );

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 2, "`Add {{R}}{{R}}`");
    assert_eq!(
        pool.total(),
        2,
        "and nothing else: the pool was empty before the activation, so both \
         red came off the Stoker's own ability"
    );

    assert!(
        is_tapped(&engine, stoker),
        "`{{T}}` was half the cost, paid by the Stoker itself"
    );
    assert!(
        on_battlefield(&engine, p0, krark_clan_stoker()).is_some(),
        "the Stoker ate an artifact and not itself"
    );
    let left = all_on_battlefield(&engine, p0, quiet_artifact());
    assert_eq!(left.len(), 1, "one of the two Sol Rings paid the price");
    assert!(
        left.contains(&rings[0]) && !left.contains(&rings[1]),
        "and it is the artifact that was not named that is still standing"
    );
    assert!(
        in_graveyard(&engine, p0, quiet_artifact()).is_some(),
        "a sacrificed permanent goes to its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "the opponent's board never moved"
    );

    // The tap is spent, so the line is no longer one the seat may take — the
    // same pool-and-permanent reading as above, one activation later.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds priority again, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(stoker, 0)),
        "a tapped Stoker has no `{{T}}` left to pay with: {:?}",
        legal.abilities
    );
}
