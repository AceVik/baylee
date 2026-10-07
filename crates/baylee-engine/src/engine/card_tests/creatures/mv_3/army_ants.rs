//! `cards/creatures/mv_3/army_ants.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Army Ants prints "{T}, Sacrifice a land: Destroy target land." — one
/// activation whose price and whose target are both lands, on opposite sides
/// of the table. The board gives p0 two Forests so the sacrifice is a real
/// choice rather than the only land in play, and p1 two lands so the
/// destruction takes the one that was named and leaves the other. Nothing
/// taps for mana anywhere: the whole price is the Ants' own `{T}` and a land,
/// so an empty pool is what makes "no mana was spent" exact, and the target is
/// answered before the sacrifice (CR 601.2c, then 601.2h) — while the question
/// stands the Ants are untapped and both Forests are still standing.
#[test]
#[allow(clippy::too_many_lines)] // one activation, both halves of its price and its target read off it
fn army_ants_eats_a_land_of_its_own_to_destroy_a_land_across_the_table() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[army_ants(), forest(), forest()])
        .battlefield(1, &[mountain(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ants = on_battlefield(&engine, p0, army_ants()).expect("the Ants are on the table");
    let mine = all_on_battlefield(&engine, p0, forest());
    assert_eq!(
        mine.len(),
        2,
        "two Forests of my own, one of which stays standing"
    );
    let (eaten, kept) = (mine[0], mine[1]);
    let their_mountain = on_battlefield(&engine, p1, mountain()).expect("their Mountain is out");
    let their_island = on_battlefield(&engine, p1, island()).expect("their Island is out");

    // The whole price is the tap symbol and a land: no mana is involved, so
    // nothing is tapped to read the offer and the pool starts empty.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the Ants' price is their own {{T}} and a land, so no mana is on this board"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(ants, 0)),
        "an untapped creature with two lands to eat pays the cost, so the one \
         line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, army_ants(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target land\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat aims it");
    assert!(
        options.contains(&their_mountain) && options.contains(&their_island),
        "\"target land\" reaches across the table: {options:?}"
    );
    assert!(
        options.contains(&eaten) && options.contains(&kept),
        "and your own lands are lands too — the filter names no controller: {options:?}"
    );
    assert!(
        !options.contains(&ants),
        "a creature is no land, and the Ants are the source besides: {options:?}"
    );
    // CR 601.2c before CR 601.2h: the target is named while nothing is paid.
    assert!(
        !is_tapped(&engine, ants),
        "no cost is paid while the target question stands"
    );
    assert!(
        in_graveyard(&engine, p0, forest()).is_none(),
        "and no land has been sacrificed yet"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![their_mountain],
            },
        )
        .expect("the Mountain was one of the options");

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
            "the sacrifice is a cost and is asked next, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat answers its own cost");
    assert_eq!(
        prompt,
        ChoicePrompt::CostSacrifice,
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one land, no more and no fewer");
    assert!(
        options.contains(&eaten) && options.contains(&kept),
        "both Forests this seat controls are on the menu: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "\"a land\" you control: the two Forests and nothing else: {options:?}"
    );
    assert!(
        !options.contains(&ants),
        "`Filter::YOUR_LAND` is read, not skipped: a creature pays nothing here: {options:?}"
    );
    assert!(
        !options.contains(&their_mountain) && !options.contains(&their_island),
        "`CR 701.21a`: the lands across the table are not yours to sacrifice: {options:?}"
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![their_island],
                },
            )
            .is_err(),
        "an answer the question did not enumerate is refused"
    );
    assert!(
        on_battlefield(&engine, p1, island()).is_some(),
        "and the refusal costs the other seat nothing"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![eaten],
            },
        )
        .expect("the Forest the question offered pays the cost");

    assert!(
        is_tapped(&engine, ants),
        "{{T}} was the other half of the price"
    );
    assert!(
        in_graveyard(&engine, p0, forest()).is_some(),
        "a sacrificed permanent goes to its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, mountain()).is_some(),
        "destroying is an effect, so the Mountain waits on the stack"
    );
    assert!(!stack_is_empty(&engine), "and the ability is on it");
    assert_eq!(
        lands_of(&engine, p0).len(),
        1,
        "exactly one land was given up: the other is still standing"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, mountain()).is_some(),
        "\"destroy target land\": the land that was named is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, island()).is_some(),
        "and the land nobody named never moved"
    );
    assert!(
        on_battlefield(&engine, p0, army_ants()).is_some(),
        "the Ants outlive the land they ate"
    );
}
