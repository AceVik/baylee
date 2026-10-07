//! `cards/creatures/mv_2/sage_of_lat_nam.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sage of Lat-Nam — {1}{U}, a 1/2 Human Artificer printing exactly one line:
/// "{T}, Sacrifice an artifact: Draw a card."
///
/// The sacrifice names no artifact of its own, so the engine has to ask which
/// one, and the menu is half the card: the Sol Ring beside it is on it, the two
/// creatures are not (the Sage is no artifact either, so the missing "another"
/// costs it nothing) and neither is the Sol Ring across the table, which
/// CR 701.21a keeps off it: nobody sacrifices what they do not control. The
/// price is a tap symbol and a permanent, so no mana is floating anywhere —
/// the offer turns on nothing the pool could have supplied, which is what
/// makes the missing entry a statement about the filter.
/// The draw is read as a *move*, and only after the ability resolves: a library
/// one shorter *and* a hand one longer, because a card that merely left the top
/// of the library would satisfy the first count on its own.
#[allow(clippy::too_many_lines)] // one activation, every gate it passes asserted
#[test]
fn sage_of_lat_nam_eats_an_artifact_of_yours_to_draw_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(41, forest())
        .battlefield(0, &[sage_of_lat_nam(), quiet_artifact(), llanowar_elves()])
        .battlefield(1, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sage = on_battlefield(&engine, p0, sage_of_lat_nam()).expect("the Sage is on the table");
    let ring = on_battlefield(&engine, p0, quiet_artifact()).expect("the Sol Ring is beside it");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("and an Elf is too");
    let theirs = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the whole price is the tap symbol and an artifact, so nothing is \
         floating to pay with"
    );
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the seat with the Sage holds it");
    assert!(
        legal.abilities.contains(&(sage, 0)),
        "an artifact to eat is on the board, so the one line the Sage prints is \
         offered: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(&mut engine, p0, sage_of_lat_nam(), 0);
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
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one artifact, no more and no fewer");
    assert_eq!(
        options,
        vec![ring],
        "the Sol Ring is the only artifact this seat controls, and nothing else \
         on the table belongs on the menu"
    );
    assert!(
        !options.contains(&sage),
        "the Sage is a creature: it cannot eat itself"
    );
    assert!(
        !options.contains(&elves),
        "and neither is an Elf an artifact"
    );
    assert!(
        !options.contains(&theirs),
        "`CR 701.21a`: an opponent's artifact is not yours to sacrifice"
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
                objects: vec![ring],
            },
        )
        .expect("the artifact the question offered pays the cost");

    // CR 601.2h: the price is the last step of the activation, so by the time
    // the draw is waiting the Sol Ring is already buried and the Sage already
    // tapped — and the card is still on the library, because drawing is the
    // *effect* and it has not resolved.
    assert!(
        in_graveyard(&engine, p0, quiet_artifact()).is_some(),
        "a sacrificed permanent goes to its owner's graveyard"
    );
    assert!(
        is_tapped(&engine, sage),
        "{{T}} is the other half of the price"
    );
    assert!(
        !stack_is_empty(&engine),
        "drawing a card is no mana ability, so the Sage's ability is waiting"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before,
        "and nothing has been drawn yet"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card left the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "and it reached the hand — a library that emptied would satisfy the \
         count above without drawing anything"
    );
    assert!(
        on_battlefield(&engine, p0, sage_of_lat_nam()).is_some(),
        "the Sage ate the Sol Ring and not itself"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "the artifact the ability did not name never moved"
    );
}
