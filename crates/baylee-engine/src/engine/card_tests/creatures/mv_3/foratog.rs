//! `cards/creatures/mv_3/foratog.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Foratog — `{2}{G}`, a printed 1/2 Atog: "`{G}`, Sacrifice a Forest: This
/// creature gets +2/+2 until end of turn."
///
/// The price names a *Forest* and not a land, so the board carries the three
/// cases a filter has to separate: two Forests under the same seat (the
/// price), the Atog itself (a permanent, and no Forest), and a Forest across
/// the table (a Forest, and not this seat's to give up). Exactly one Forest is
/// tapped for the `{G}` and the other, left standing, is the one eaten — so
/// the pool reads empty once the ability is on the stack and the green in it
/// belonged to the mana rather than to change. `(3, 4)` on a printed `(1, 2)`
/// is the only body that reads `+2/+2` as a pump on this creature, and the
/// typed cost is what makes the offer worth playing: reading the card file
/// cannot tell "a Forest you control" from "a permanent you control".
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn foratog_eats_a_forest_of_your_own_for_two_and_two() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[foratog(), forest(), forest()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let toad = on_battlefield(&engine, p0, foratog()).expect("the Atog is on the table");
    let lands = all_on_battlefield(&engine, p0, forest());
    assert_eq!(lands.len(), 2, "two Forests of its own to give up");
    let (food, fuel) = (lands[0], lands[1]);
    let theirs = on_battlefield(&engine, p1, forest()).expect("a Forest across the table");
    assert_eq!(
        pt(&engine, toad),
        (1, 2),
        "a printed 1/2 before anything is eaten"
    );

    // `legal.abilities` is filtered through `can_afford`, and that reads the
    // pool rather than the untapped lands: no {G}, no offer.
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the seat with the Atog");
    assert!(
        !legal.abilities.contains(&(toad, 0)),
        "an empty pool pays no {{G}}, and an unaffordable ability is absent from \
         the offer rather than refused: {:?}",
        legal.abilities
    );

    // One Forest pays the {G} and the other is written down as the one that
    // stays untapped: the Atog is a 1/2 and taps for nothing, so nothing else
    // on this board can move the pool between here and the claim.
    tap_mana_except(&mut engine, p0, food);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one Forest in the pool and the other kept back for the sacrifice"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(toad, 0)),
        "with {{G}} floating the whole price is payable: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, foratog(), 0);
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("the cost asks which Forest, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat answers its own cost");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one Forest, no more and no fewer");
    assert_eq!(
        options.len(),
        2,
        "the two Forests this seat controls and nothing else: {options:?}"
    );
    assert!(
        options.contains(&food) && options.contains(&fuel),
        "a tapped Forest is still a Forest, so both are on the menu: {options:?}"
    );
    assert!(
        !options.contains(&toad),
        "the Atog is a creature: \"a Forest\" is read, not skipped: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "`CR 701.21a`: an opponent's Forest is not yours to sacrifice: {options:?}"
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
        on_battlefield(&engine, p1, forest()).is_some(),
        "and the refusal costs the other seat nothing"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![food],
            },
        )
        .expect("the Forest the question offered pays the cost");

    // CR 601.2h: the {G} and the sacrifice are the last step of the
    // activation, so both are already paid while the ability waits to resolve.
    assert!(
        in_graveyard(&engine, p0, forest()).is_some(),
        "a sacrificed Forest goes to its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{G}} was the other half of the price"
    );
    assert_eq!(
        pt(&engine, toad),
        (1, 2),
        "the pump is an effect and resolves off the stack, so the body has not \
         moved yet"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, toad),
        (3, 4),
        "+2/+2 until end of turn on the creature the ability belongs to"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, forest()),
        vec![fuel],
        "exactly one Forest was given up: the other is still standing"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "and nothing of the opponent's ever moved"
    );
}
