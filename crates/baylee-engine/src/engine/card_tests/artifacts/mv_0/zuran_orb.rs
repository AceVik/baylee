//! `cards/artifacts/mv_0/zuran_orb.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Zuran Orb costs `{0}` and prints one line: "Sacrifice a land: You gain 2
/// life." The sacrifice names no land in particular, so the engine has to ask
/// which one — and that menu is half the card: both lands this seat controls
/// are on it, while the Orb itself is an artifact and no land (`Filter::
/// YOUR_LAND` is read, not skipped) and the Forest across the table is not
/// this seat's to give up (CR 701.21a).
///
/// The other half is the ordering CR 601.2h gives every activation: the land
/// is already in the graveyard *before* the ability goes on the stack, so the
/// two life can only arrive when it resolves — no mana on this board could
/// have bought it, and the pool is asserted empty to say so.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn zuran_orb_eats_a_land_of_your_own_for_two_life() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), island()])
        .hand(0, &[zuran_orb()])
        .battlefield(1, &[forest()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // `{0}` is affordable on an empty board, so nothing is tapped to pay for
    // the artifact and the two lands under p0 are exactly what the sacrifice
    // is about to choose between.
    cast_with_floating(&mut engine, p0, zuran_orb());
    pass_until(&mut engine, stack_is_empty);
    let orb = on_battlefield(&engine, p0, zuran_orb()).expect("the Orb resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "a zero-cost artifact spends nothing"
    );

    let lands = lands_of(&engine, p0);
    assert_eq!(lands.len(), 2, "two lands to choose between");
    let mine = lands[0];
    let kept = lands[1];
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "and one across the table that is not this seat's to give up"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(orb, 0)),
        "the Orb's only line costs a land and no mana, so it is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, zuran_orb(), 0);
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("the cost asks which land, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat is the one asked");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one land, no more and no fewer");
    assert_eq!(
        options.len(),
        2,
        "the two lands this seat controls and nothing else: {options:?}"
    );
    assert!(
        options.contains(&mine) && options.contains(&kept),
        "both are lands you control: {options:?}"
    );
    assert!(
        !options.contains(&orb),
        "the Orb is an artifact: it cannot eat itself: {options:?}"
    );
    assert!(
        !options
            .contains(&on_battlefield(&engine, p1, forest()).expect("their Forest still stands")),
        "a seat sacrifices only what it controls, whatever the filter says: {options:?}"
    );

    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![
                        on_battlefield(&engine, p1, forest()).expect("their Forest still stands")
                    ],
                },
            )
            .is_err(),
        "an answer the question did not enumerate is refused"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "and the refusal costs the other seat nothing"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the land the question offered pays the cost");

    assert!(
        in_graveyard(&engine, p0, forest()).is_some(),
        "CR 601.2h: the price is paid before the ability is on the stack, so \
         the land is already in its owner's graveyard"
    );
    assert!(
        !stack_is_empty(&engine),
        "and the ability is what is waiting: it is no mana ability"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "nothing has been gained yet — the effect resolves off the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 22, "\"You gain 2 life.\"");
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the life belongs to the player who paid, not the opponent"
    );
    assert_eq!(
        lands_of(&engine, p0).len(),
        1,
        "exactly one land was given up: the other is still standing"
    );
    assert!(
        on_battlefield(&engine, p0, zuran_orb()).is_some(),
        "the Orb outlives the land it ate"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "and nothing of the opponent's ever moved"
    );
}
