//! `cards/creatures/mv_3/phyrexian_broodlings.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Phyrexian Broodlings prints exactly one line — "{1}, Sacrifice a creature:
/// Put a +1/+1 counter on this creature." — and its two halves land in
/// different places, so one activation has to be read in three: the {1} leaves
/// the pool, the creature named at the cost menu leaves the battlefield for
/// its owner's graveyard, and the counter lands on the *source* rather than on
/// whatever was given up. The menu is half the card, and both of its edges are
/// struck here: `Filter::YOUR_CREATURE` offers both creatures this seat
/// controls — the Broodlings among them, because the source is still on the
/// battlefield when CR 601.2h asks — and declines the Elf across the table,
/// which CR 701.21a keeps off it whatever the filter says. The surviving
/// bystander is the third reading: a `+1/+1` on a printed 2/2 is a 3/3, so a
/// counter that never arrived would leave the pair at 2/2 and 1/1 with one
/// creature missing from the board.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn phyrexian_broodlings_eats_a_creature_of_yours_for_a_counter_on_itself() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[phyrexian_broodlings(), quiet_creature(), swamp()])
        .battlefield(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let broodlings =
        on_battlefield(&engine, p0, phyrexian_broodlings()).expect("the Broodlings is seated");
    let fodder = on_battlefield(&engine, p0, quiet_creature()).expect("a creature to give up");
    let theirs = on_battlefield(&engine, p1, quiet_creature()).expect("their creature is out");
    assert_eq!(
        pt(&engine, broodlings),
        (2, 2),
        "the printed 2/2 body, before anything is paid"
    );
    assert_eq!(
        counters_on(&engine, broodlings, CounterKind::P1P1),
        0,
        "and nothing has been put on it yet"
    );

    // `legal.abilities` is filtered through `can_afford`, which reads the
    // *pool* and not the untapped lands, so the {1} has to be floating before
    // anything is claimed about the offer. The Elf is named as the printing
    // kept back: it is the creature this test sacrifices, and a source tapped
    // for mana is a source whose status has already changed for a reason of
    // its own.
    tap_all_mana_but(&mut engine, p0, Some(quiet_creature()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one Swamp tapped, and no creature paid in"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(broodlings, 0)),
        "the one line the card prints is offered with {{1}} floating: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, phyrexian_broodlings(), 0);
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
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one creature, no more and no fewer");
    assert!(
        options.contains(&fodder),
        "the creature beside it is on the menu: {options:?}"
    );
    assert!(
        options.contains(&broodlings),
        "and so is the Broodlings itself: \"a creature\" is every creature \
         this seat controls, and the source is still on the battlefield while \
         the cost is being chosen (CR 601.2h): {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "those two are the whole menu: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "`CR 701.21a`: an opponent's creature is not yours to sacrifice: {options:?}"
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
        on_battlefield(&engine, p1, quiet_creature()).is_some(),
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

    // CR 601.2h: the {1} goes out of the pool as the last step of the
    // activation, and putting a counter on a creature is no mana ability.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "the ability is on the stack, waiting to resolve"
    );
    assert_eq!(
        counters_on(&engine, broodlings, CounterKind::P1P1),
        0,
        "and the counter has not landed yet"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, broodlings, CounterKind::P1P1),
        1,
        "\"Put a +1/+1 counter on this creature\" — one counter, on the source"
    );
    assert_eq!(
        pt(&engine, broodlings),
        (3, 3),
        "a printed 2/2 with one +1/+1 counter on it"
    );
    assert!(
        on_battlefield(&engine, p0, phyrexian_broodlings()).is_some(),
        "the creature that took the counter is still on the battlefield, which \
         is what says it was not the one sacrificed"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_creature()).is_none(),
        "the sacrificed creature left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, quiet_creature()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_creature()).is_some(),
        "the creature the ability did not name never moved"
    );
}
