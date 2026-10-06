//! `cards/enchantments/mv_2/trade_routes.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Trade Routes — {1}{U} enchantment — prints two activated abilities and one
/// board plays both: three Islands pay the cast, and the two `{1}`s that follow
/// come out of the three Forests the cast was told to leave standing. The first
/// line, "`{1}`: Return target land you control to its owner's hand", is read as
/// a *move* — the tapped Island the cast spent leaves the battlefield and is
/// offered as a land drop again — and the second, "`{1}`, Discard a land card:
/// Draw a card", spends that very Island, so the card one line hands back is the
/// card the other costs. The Forest across the table is the control on both
/// "you"s: it is a land, it is not this seat's, and neither ability may touch it.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn trade_routes_bounces_a_land_then_trades_one_for_a_card() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[island(), island(), island(), forest(), forest(), forest()],
        )
        .hand(0, &[trade_routes(), counterspell()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // **Two** named Islands pay the {1}{U} and not every Island standing:
    // the pool has to empty on the cast, because "neither ability is
    // affordable yet" two lines down is a claim about the pool and a
    // leftover mana would answer it by accident. The Forests are the two
    // {1}s the card's own abilities charge afterwards.
    let islands = all_on_battlefield(&engine, p0, island());
    assert_eq!(islands.len(), 3, "three Islands were dealt");
    let (first, second) = (islands[0], islands[1]);
    tap_mana_where(&mut engine, p0, |id| id == first || id == second);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Islands tapped, which is the {{1}}{{U}} Trade Routes costs"
    );
    cast_with_floating(&mut engine, p0, trade_routes());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let routes = on_battlefield(&engine, p0, trade_routes()).expect("Trade Routes resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the cast spent the pool, so neither ability is affordable yet"
    );
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert_eq!(player, p0, "and it is the seat with the enchantment");
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == routes),
        "an empty pool pays no {{1}}, and an unaffordable ability is absent \
         from the offer rather than refused: {:?}",
        legal.abilities
    );

    let bounced = islands[0];
    let forests = all_on_battlefield(&engine, p0, forest());
    assert_eq!(forests.len(), 3, "three Forests were dealt");
    let theirs = on_battlefield(&engine, p1, forest()).expect("a Forest across the table");

    // The two named Forests and nothing else: the third Island is standing
    // too, and "everything but one" would have floated it as well.
    let (left, right) = (forests[1], forests[2]);
    let taken = tap_mana_where(&mut engine, p0, |id| id == left || id == right);
    assert_eq!(taken, 2, "the two Forests beside the one kept back");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two green, one for each {{1}}"
    );

    // Ability 0: "{1}: Return target land you control to its owner's hand."
    activate(&mut engine, p0, trade_routes(), 0);
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target land you control\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat chooses");
    assert_eq!((min, max), (1, 1), "exactly one land");
    assert!(
        options.contains(&bounced),
        "a tapped land you control is still a land you control: {options:?}"
    );
    assert!(
        options.contains(&forests[0]),
        "and so is the untapped one: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "\"you control\" declines the Forest across the table: {options:?}"
    );
    assert!(
        !options.contains(&routes),
        "the enchantment is no land: {options:?}"
    );
    assert_eq!(
        options.len(),
        6,
        "the six lands this seat controls and nothing else: {options:?}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "CR 601.2h pays last, so the {{1}} is still in the pool while the \
         question stands"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bounced],
            },
        )
        .expect("the land the question offered is the one that goes back");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{1}} came out of the pool"
    );
    assert!(!stack_is_empty(&engine), "and the ability is on the stack");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(
        engine.state().object(bounced).map(|o| o.zone),
        Some(Zone::Hand),
        "the land left the battlefield for its owner's hand"
    );
    assert!(
        in_hand(&engine, p0, island()).is_some(),
        "and it is this seat's hand that holds it"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, island()).len(),
        2,
        "two Islands are still standing"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "the Forest the ability did not name never moved"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.lands.contains(&bounced),
        "\"return it to your hand\" means it is a land drop again: {:?}",
        legal.lands
    );

    let library_before = library_size(&engine, p0);
    let top = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(p0))
        .last()
        .expect("p0 has a library");
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let spell = in_hand(&engine, p0, counterspell()).expect("a nonland card in hand");

    // Ability 1: "{1}, Discard a land card: Draw a card."
    activate(&mut engine, p0, trade_routes(), 1);
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
            "the discard is a cost and is asked before it is paid: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat answers its own cost");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostDiscard,
        "a cost and not a search, which is all a client has to tell them apart"
    );
    assert_eq!((min, max), (1, 1), "one land card, no more and no fewer");
    assert!(
        options.contains(&bounced),
        "the Island the first line handed back is a land card in hand: {options:?}"
    );
    assert!(
        !options.contains(&spell),
        "\"a land card\" is read and not skipped: the Counterspell in hand is \
         not on the menu: {options:?}"
    );
    assert!(
        options
            .iter()
            .all(|id| types(&engine, *id).contains(TypeSet::LAND)),
        "and nothing but land cards is: {options:?}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{1}} is not paid until the question is answered"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bounced],
            },
        )
        .expect("the card the question offered pays the cost");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the second {{1}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "drawing a card is no mana ability"
    );
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        in_graveyard(&engine, p0, island()).is_some(),
        "the discarded land card went to its owner's graveyard"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\""
    );
    assert_eq!(
        engine.state().object(top).map(|o| o.zone),
        Some(Zone::Hand),
        "and the card drawn is the one that was on top of the library, not \
         merely some card that appeared in hand"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "one card discarded and one drawn, so the hand is the size it was"
    );
}
