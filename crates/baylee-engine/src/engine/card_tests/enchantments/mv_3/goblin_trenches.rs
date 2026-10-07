//! `cards/enchantments/mv_3/goblin_trenches.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Goblin Trenches prints a single line: "{2}, Sacrifice a land: Create
/// two 1/1 red and white Goblin Soldier creature tokens." Both parts of the
/// cost are readable on the same battlefield — five lands pay the
/// `{1}{R}{W}` of the enchantment and leave floating in the same main phase
/// (CR 500.5) exactly the `{2}` that the ability then requires, so the empty
/// pool afterwards proves the payment and not just a label. The sacrificed
/// land is the *own* one: the Forest on the table is on no option list
/// (CR 701.21a), and because the sacrifice is the last step of the
/// activation (CR 601.2h), it is only in the graveyard after the response.
/// The two Soldiers are the yield that no mana calculation can predict.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn goblin_trenches_eats_a_land_of_your_own_for_two_goblin_soldiers() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[goblin_trenches()])
        .battlefield(0, &[mountain(), plains(), forest(), forest(), forest()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Five lands make the `{1}{R}{W}` of the enchantment, and because
    // CR 500.5 empties the pool only at the end of a step, the `{2}` that
    // the ability costs stays in this one main phase.
    cast_from_hand(&mut engine, p0, goblin_trenches());
    pass_until(&mut engine, stack_is_empty);
    let trenches = on_battlefield(&engine, p0, goblin_trenches()).expect("the Trenches resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "five lands paid the {{1}}{{R}}{{W}} and the ability's {{2}} is still in the pool"
    );

    // `legal.abilities` is filtered behind `can_afford`, and that reads the
    // pool: the line is only now offered at all.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(trenches, 0)),
        "the one line the card prints, now that its {{2}} is payable: {:?}",
        legal.abilities
    );

    let mine = lands_of(&engine, p0);
    assert_eq!(mine.len(), 5, "five lands, all of them still on the table");
    let victim = all_on_battlefield(&engine, p0, forest())[0];
    let theirs = on_battlefield(&engine, p1, forest()).expect("their Forest is out");

    activate(&mut engine, p0, goblin_trenches(), 0);
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
            "the sacrifice is chosen before it is paid: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat answers its own cost");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one land, no more and no fewer");
    for land in &mine {
        assert!(
            options.contains(land),
            "every land this seat controls is on the menu: {options:?}"
        );
    }
    assert_eq!(options.len(), 5, "and those five are the whole menu");
    assert!(
        !options.contains(&trenches),
        "the Trenches are an enchantment: it cannot eat itself: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "`CR 701.21a`: an opponent's land is not yours to sacrifice: {options:?}"
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
                objects: vec![victim],
            },
        )
        .expect("the land the question offered pays the cost");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "CR 601.2h: the {{2}} came out of the pool as the last step of the \
         activation"
    );
    assert!(
        in_graveyard(&engine, p0, forest()).is_some(),
        "and the sacrificed land is in its owner's graveyard, not merely gone"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        lands_of(&engine, p0).len(),
        4,
        "exactly one land was given up: the other four are still standing"
    );
    let tokens = tokens_of(&engine, p0);
    assert_eq!(
        tokens.len(),
        2,
        "\"create two 1/1 red and white Goblin Soldier creature tokens\""
    );
    for id in &tokens {
        let chars = engine
            .state()
            .object(*id)
            .expect("the token is on the battlefield")
            .characteristics();
        assert!(
            chars.types.contains(TypeSet::CREATURE),
            "a token that is not a creature could not attack: {chars:?}"
        );
        assert_eq!(
            (chars.power, chars.toughness),
            (Some(1), Some(1)),
            "a 1/1 body, on each of the two"
        );
        let soldier = engine
            .state()
            .object(*id)
            .and_then(|o| o.token)
            .expect("it knows which token it is");
        assert!(
            soldier.colors.contains(baylee_core::color::Color::Red)
                && soldier.colors.contains(baylee_core::color::Color::White),
            "red and white, the color pair the card prints"
        );
    }
    assert!(
        on_battlefield(&engine, p0, goblin_trenches()).is_some(),
        "the enchantment outlives the land it ate"
    );
}
