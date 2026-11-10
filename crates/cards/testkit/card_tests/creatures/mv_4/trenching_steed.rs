//! `cards/creatures/mv_4/trenching_steed.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
#[allow(clippy::too_many_lines)] // one activation, and every part of its price read off a different zone
fn trenching_steed_eats_a_land_of_yours_for_three_toughness_until_the_turn_ends() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains(), plains(), forest()])
        .hand(0, &[trenching_steed()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Four Plains pay {3}{W} to the last mana, and the fifth land is kept
    // back: it is the one the ability is about to eat, and a land tapped for
    // mana is a land whose status has already changed for a reason of its own.
    tap_all_mana_but(&mut engine, p0, Some(forest()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Plains, and the Forest untouched"
    );
    cast_with_floating(&mut engine, p0, trenching_steed());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{3}}{{W}} came out of the pool"
    );
    let steed = on_battlefield(&engine, p0, trenching_steed()).expect("the Steed resolved");
    assert_eq!(pt(&engine, steed), (2, 3), "the body the card prints");
    let fodder = on_battlefield(&engine, p0, forest()).expect("my Forest is out");
    let theirs = on_battlefield(&engine, p1, forest()).expect("their Forest is out");

    // The ability costs no mana and no tap, so an empty pool withholds
    // nothing: both the lands and the Steed are on the board.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(steed, 0)),
        "the one line the card prints, with five lands to pay it: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, trenching_steed(), 0);
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
    assert_eq!(player, p0, "the activating seat answers its own cost");
    assert_eq!(
        prompt,
        ChoicePrompt::CostSacrifice,
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one land, no more and no fewer");
    assert_eq!(
        options.len(),
        5,
        "the five lands this seat controls and nothing else: {options:?}"
    );
    assert!(
        options.contains(&fodder),
        "\"a land\" is every land you control, tapped or not: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "`CR 701.21a`: an opponent's land is not yours to sacrifice: {options:?}"
    );
    assert!(
        !options.contains(&steed),
        "the Steed is a creature and no land, so it cannot pay its own price: {options:?}"
    );

    let refused = engine.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![theirs],
        },
    );
    assert!(
        refused.is_err(),
        "an answer the question did not enumerate is refused: {refused:?}"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "and the refusal costs the other seat nothing"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .expect("the land the question offered pays the cost");

    assert!(
        in_graveyard(&engine, p0, forest()).is_some(),
        "CR 601.2h: the price is paid before the ability is on the stack"
    );
    assert!(
        !stack_is_empty(&engine),
        "and the pump is what is waiting: it is no mana ability"
    );
    assert_eq!(
        pt(&engine, steed),
        (2, 3),
        "so nothing has been added while the question is still open"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, steed),
        (2, 6),
        "\"+0/+3\" on the creature the ability names — power untouched"
    );
    assert!(
        on_battlefield(&engine, p0, trenching_steed()).is_some(),
        "the sacrifice was the land and not the Steed"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "and nothing of the opponent's ever moved"
    );

    // "until end of turn" is part of the card: a turn later the creature is
    // still standing and the three toughness are gone.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, steed),
        (2, 3),
        "the pump lasted the turn it was made in and no longer"
    );
}
