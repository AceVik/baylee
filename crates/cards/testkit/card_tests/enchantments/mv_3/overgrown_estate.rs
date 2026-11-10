//! `cards/enchantments/mv_3/overgrown_estate.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Overgrown Estate — {W}{B}{G} enchantment: "Sacrifice a land: You gain 3
/// life."
///
/// The sacrifice names no land in particular, so the engine has to ask which
/// one — and that menu is half the card: every land this seat controls is on
/// it, while the Estate itself is an enchantment and no land, and the Forest
/// across the table is not this seat's to give up (CR 701.21a). The other half
/// is the ordering CR 601.2h gives every activation: the land is already in
/// its owner's graveyard *before* the ability goes on the stack, so the three
/// life can only arrive when it resolves — no mana on this board could have
/// bought it, and the pool is asserted empty to say so.
#[allow(clippy::too_many_lines)] // one activation, every part of its price read off a different zone
#[test]
fn overgrown_estate_eats_a_land_of_your_own_for_three_life() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), swamp(), forest()])
        .hand(0, &[overgrown_estate()])
        .battlefield(1, &[forest()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {W}{B}{G} off one of each, which leaves the pool empty once the
    // enchantment has landed: the price below is a land and no mana at all.
    cast_from_hand(&mut engine, p0, overgrown_estate());
    pass_until(&mut engine, stack_is_empty);
    let estate = on_battlefield(&engine, p0, overgrown_estate()).expect("the Estate resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the three lands paid the {{W}}{{B}}{{G}} exactly and float nothing"
    );

    let my_forest = on_battlefield(&engine, p0, forest()).expect("my Forest is out");
    let their_forest = on_battlefield(&engine, p1, forest()).expect("their Forest is out");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(estate, 0)),
        "the Estate's only line costs a land and no mana, so it is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, overgrown_estate(), 0);
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
        ChoicePrompt::CostSacrifice,
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one land, no more and no fewer");
    assert_eq!(
        options.len(),
        3,
        "the three lands this seat controls and nothing else: {options:?}"
    );
    assert!(
        options.contains(&my_forest),
        "a land you control is on the menu: {options:?}"
    );
    assert!(
        !options.contains(&estate),
        "the Estate is an enchantment and no land: {options:?}"
    );
    assert!(
        !options.contains(&their_forest),
        "CR 701.21a: an opponent's land is not yours to sacrifice: {options:?}"
    );

    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![their_forest],
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
                objects: vec![my_forest],
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
        "and the ability is what is waiting: gaining life is no mana ability"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "nothing has been gained yet — the effect resolves off the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 23, "\"You gain 3 life.\"");
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the life belongs to the player who paid, not the opponent"
    );
    assert_eq!(
        lands_of(&engine, p0).len(),
        2,
        "exactly one land was given up: the other two are still standing"
    );
    assert!(
        on_battlefield(&engine, p0, overgrown_estate()).is_some(),
        "the Estate outlives the land it ate"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "and nothing of the opponent's ever moved"
    );
}
