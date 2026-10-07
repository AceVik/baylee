//! `cards/creatures/mv_2/oboro_breezecaller.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Oboro Breezecaller — {1}{U}, a 1/1 Moonfolk Wizard with flying — prints
/// "{2}, Return a land you control to its owner's hand: Untap target land."
/// One activation reads every part of that off a different place: the {2}
/// leaves a pool only the three Forests paid into, the returned land is the
/// *cost* and is in its owner's hand before the ability resolves, and the
/// untap is the effect — so the land that was named is still tapped while the
/// price is being settled (CR 601.2c before CR 601.2h). A third Forest that
/// is neither aimed at nor given up stays tapped, which is what says "untap
/// target land" is one land and not a board untap, and the Forest across the
/// table is offered as a target and never moves.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn oboro_breezecaller_returns_a_land_to_untap_the_land_it_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), oboro_breezecaller()])
        // A land on the other side of the table: "target land" is any land,
        // and a filter narrowed to your own would still offer three.
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let caller =
        on_battlefield(&engine, p0, oboro_breezecaller()).expect("the Breezecaller is out");
    assert!(
        keywords(&engine, caller).contains(KeywordSet::FLYING),
        "the printed flying line reaches the creature"
    );
    let forests = all_on_battlefield(&engine, p0, forest());
    assert_eq!(
        forests.len(),
        3,
        "three Forests: two to pay the {{2}} with and one to give up"
    );
    let (aimed, given_up, bystander) = (forests[0], forests[1], forests[2]);
    let theirs = on_battlefield(&engine, p1, forest()).expect("a Forest across the table");

    // The pool is what `can_afford` reads and not the untapped lands, so the
    // Forests are tapped before anything is claimed about the offer. The
    // Breezecaller prints no mana ability, so it is none of the routes taken.
    assert_eq!(
        tap_all_mana(&mut engine, p0),
        3,
        "the three Forests, and nothing else on the board makes mana"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three green floating"
    );
    assert!(
        forests.iter().all(|id| is_tapped(&engine, *id)),
        "and every Forest is down"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(caller, 0)),
        "with {{2}} floating and a land to give up, the one line the card \
         prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, oboro_breezecaller(), 0);

    // Two questions, answered in the order they arrive: the target
    // (CR 601.2c) and then the land the cost gives up (CR 601.2h).
    let mut asked_target = false;
    let mut asked_cost = false;
    for _ in 0..12 {
        if asked_target && asked_cost {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player,
                options,
                min,
                max,
                ..
            } => {
                assert_eq!(player, p0, "the activating seat aims it");
                assert_eq!((min, max), (1, 1), "one land, no more and no fewer");
                assert!(
                    options.contains(&aimed),
                    "a land this seat controls is on the menu: {options:?}"
                );
                assert!(
                    options.contains(&theirs),
                    "\"target land\" is any land, on either side of the table: {options:?}"
                );
                assert!(
                    !options.contains(&caller),
                    "a Wizard is no land: {options:?}"
                );
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![aimed],
                        },
                    )
                    .expect("the land the question offered is a legal target");
                asked_target = true;
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                prompt,
                ..
            } => {
                assert_eq!(player, p0, "the activating seat pays its own cost");
                assert_eq!(
                    prompt,
                    ChoicePrompt::CostReturn,
                    "a cost and not a search, and the variant is what tells a \
                     client the land is coming back rather than being given up"
                );
                assert_eq!((min, max), (1, 1), "one land, no more and no fewer");
                assert!(
                    options.contains(&given_up),
                    "a land this seat controls may pay: {options:?}"
                );
                assert!(
                    !options.contains(&theirs),
                    "an opponent's land is not yours to return, whatever the \
                     filter says: {options:?}"
                );
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![given_up],
                        },
                    )
                    .expect("the land the cost offered pays for it");
                asked_cost = true;
            }
            other => {
                panic!("unexpected while the Breezecaller's activation resolves: {other:?}")
            }
        }
    }
    assert!(
        asked_target && asked_cost,
        "both the target and the cost are asked before the ability resolves"
    );

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{2}} came out of the three green the Forests filled"
    );
    assert_eq!(
        engine.state().object(given_up).map(|o| o.zone),
        Some(Zone::Hand),
        "\"return a land you control to its owner's hand\": the very land the \
         cost offered, and not a copy of the same printing"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, forest()).len(),
        2,
        "and only the land that was named — the other two Forests are still out"
    );
    assert!(
        is_tapped(&engine, aimed),
        "the untap is the effect, so the target is still tapped while the cost \
         is being settled (CR 601.2h)"
    );
    assert!(
        !stack_is_empty(&engine),
        "and the ability is what is waiting"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        !is_tapped(&engine, aimed),
        "\"untap target land\" — the land the ability was aimed at stands back up"
    );
    assert!(
        is_tapped(&engine, bystander),
        "and the Forest it was not aimed at stays down, so the effect is one \
         land and not a board untap"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_some() && !is_tapped(&engine, theirs),
        "the Forest across the table was offered as a target and was never the \
         one that was chosen"
    );
}
