//! `cards/creatures/mv_2/wall_of_mulch.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Wall of Mulch prints two lines: "Defender" on a 0/4 Wall and
/// "`{G}`, Sacrifice a Wall: Draw a card."
///
/// The cost names no specific Wall, so the engine must ask, and
/// this question is half the card. Both halves of the filter need their
/// own bystander: the three Forests are permanent cards under this
/// control that are not Walls, and the Wall of Mulch on the other
/// side of the table is a Wall that this seat does not control — a
/// menu that allowed one of the two would still have offered something and
/// still passed. The Wall that the test itself brought into play is paid;
/// the drawn card is read as movement — library one shorter, hand one
/// longer — and not as a question posed.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn wall_of_mulch_eats_a_wall_you_control_for_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .battlefield(1, &[wall_of_mulch()])
        .hand(0, &[wall_of_mulch()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Three Forests in the pool: the {1}{G} for the Wall and the {G} that the
    // ability then requires. CR 500.5 empties a pool only at the end of a
    // step, and the whole scenario plays in this one main phase.
    cast_from_hand(&mut engine, p0, wall_of_mulch());
    pass_until(&mut engine, stack_is_empty);
    let wall = on_battlefield(&engine, p0, wall_of_mulch()).expect("the Wall resolved");
    let theirs = on_battlefield(&engine, p1, wall_of_mulch()).expect("their Wall is out");
    assert_eq!(pt(&engine, wall), (0, 4), "the printed 0/4 body");
    assert!(
        keywords(&engine, wall).contains(KeywordSet::DEFENDER),
        "and the printed defender reaches the permanent"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{1}}{{G}} is spent and the ability's {{G}} is still floating"
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(wall, 0)),
        "the one line the card prints is offered now that its {{G}} is in the \
         pool: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, wall_of_mulch(), 0);
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
    assert_eq!((min, max), (1, 1), "one Wall, no more and no fewer");
    assert!(
        options.contains(&wall),
        "the Wall this seat cast is a Wall it controls, so it is on its own \
         menu: {options:?}"
    );
    assert_eq!(
        options.len(),
        1,
        "and nothing else is: the Forests are no Walls, and the Wall across \
         the table is not this seat's to give up: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "`CR 701.21a`: a Wall an opponent controls is not yours to sacrifice: {options:?}"
    );
    assert!(
        on_battlefield(&engine, p0, wall_of_mulch()).is_some(),
        "nothing is gone while the question still stands"
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
        on_battlefield(&engine, p1, wall_of_mulch()).is_some(),
        "and the refusal costs the other seat nothing"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wall],
            },
        )
        .expect("the Wall the question offered pays the cost");
    assert!(
        !stack_is_empty(&engine),
        "drawing a card is no mana ability, so the ability is on the stack"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{G}} went with it"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, wall_of_mulch()).is_none(),
        "the sacrificed Wall left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, wall_of_mulch()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\" — one off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "and it is in hand, so a library that emptied would not satisfy the \
         count above"
    );
    assert!(
        on_battlefield(&engine, p1, wall_of_mulch()).is_some(),
        "the Wall across the table never moved"
    );
}
