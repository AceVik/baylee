//! `cards/lands/utility/abstergo_entertainment.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Abstergo Entertainment prints `{{T}}: Add {{C}}.`, `{{1}}, {{T}}: Add one mana of
/// any color.`, and `{{3}}, {{T}}, Exile Abstergo Entertainment: Return up to one target
/// historic card from your graveyard to your hand, then exile all graveyards.`
///
/// "Up to one target" may name nothing (CR 115.6), so ability 2 is offered on an
/// empty graveyard. It was withheld there while an activated ability's target was a
/// bare spec, which reads as exactly one. With three green mana floating and Abstergo
/// untapped, ability 2 is offered with no historic card anywhere. Activating ability 1
/// spends one floating green mana, prompts for a color choice via `Pending::ChooseColor`,
/// adds one black mana, and leaves the land tapped.
#[test]
fn abstergo_entertainment_filters_mana_and_offers_its_exile_on_an_empty_graveyard() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[abstergo_entertainment(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let abstergo =
        on_battlefield(&engine, p0, abstergo_entertainment()).expect("abstergo on battlefield");

    // Float {3} from the three Forests while keeping Abstergo untapped.
    tap_mana_except(&mut engine, p0, abstergo);
    assert_eq!(engine.state().players[0].mana_pool.total(), 3);
    assert!(!is_tapped(&engine, abstergo));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(abstergo, 2)),
        "with {{3}} floating and {{T}} available, ability 2 is offered: up to one target \
         may be none"
    );
    assert!(legal.abilities.contains(&(abstergo, 1)));

    activate(&mut engine, p0, abstergo_entertainment(), 1);
    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!("expected ChooseColor, got {:?}", engine.pending());
    };
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1);
    assert_eq!(pool.available(ManaColor::Green), 2);
    assert_eq!(pool.total(), 3);
    assert!(is_tapped(&engine, abstergo));
}

/// Abstergo Entertainment is a legendary land printing three abilities —
/// "{T}: Add {C}", "{1}, {T}: Add one mana of any color", and "{3}, {T}, Exile
/// Abstergo Entertainment: Return up to one target historic card from your
/// graveyard to your hand, then exile all graveyards." — and all three are
/// played, one per turn of p0's, because the first two spend the single `{T}`
/// the third then needs.
///
/// The filler deck is Sol Ring, an artifact and so historic (CR 700.6), which
/// is what `seed_graveyard` puts into *both* graveyards: the target filter is
/// `PlayerRel::You`, so only my own graveyard may be named, and the sentence
/// after the return has to empty the opponent's graveyard as well.
#[test]
#[allow(clippy::too_many_lines)] // three printed abilities, each wanting the {T} the others spend
fn abstergo_entertainment_makes_mana_and_trades_itself_for_a_historic_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, quiet_artifact())
        .battlefield(0, &[abstergo_entertainment(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land =
        on_battlefield(&engine, p0, abstergo_entertainment()).expect("the land is on the table");
    assert!(!is_tapped(&engine, land), "a land enters untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats before anything has been activated"
    );

    // ---- "{T}: Add {C}" -------------------------------------------------
    //
    // `LegalActions::abilities` is filtered through `can_afford`, which reads
    // the pool and not the untapped lands — so the two printed mana lines can
    // be told apart on an empty pool: the tap is payable and the {1} is not.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(land, 0)),
        "an untapped land is a paid {{T}}, so \"{{T}}: Add {{C}}\" is offered: {:?}",
        legal.abilities
    );
    assert!(
        !legal.abilities.contains(&(land, 1)),
        "and \"{{1}}, {{T}}: Add one mana of any color\" is not, because the \
         pool is empty: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, abstergo_entertainment(), 0);
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1,
        "\"{{T}}: Add {{C}}\" — one colourless, and colourless is what the card prints"
    );
    assert!(
        is_tapped(&engine, land),
        "the tap symbol was the whole price"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == land),
        "a tapped land has no {{T}} left to pay either printed line with: {:?}",
        legal.abilities
    );

    // ---- "{1}, {T}: Add one mana of any color" ---------------------------
    //
    // One turn later the `{T}` is free again and the pool emptied with the
    // step that ended (CR 500.5), which is what makes the {1} a real price.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        on_battlefield(&engine, p0, abstergo_entertainment()).is_some(),
        "the land survived its own untap step"
    );
    assert!(
        !is_tapped(&engine, land),
        "and the untap step stood it back up"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "last turn's colourless mana is gone with the step that ended (CR 500.5)"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.contains(&(land, 1)),
        "still no {{1}} to pay with, so the coloured line is absent from the \
         offer rather than refused: {:?}",
        legal.abilities
    );

    // Three Forests pay the {1}; the land itself is named as the printing kept
    // back, since `tap_all_mana` would spend the very `{T}` under test (#159).
    tap_all_mana_but(&mut engine, p0, Some(abstergo_entertainment()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Forests, three green, and nothing off the land itself"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(land, 1)),
        "with the {{1}} already floating the coloured line is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, abstergo_entertainment(), 1);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("\"any color\" is a question, got {:?}", engine.pending());
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options.len(),
        5,
        "the five colours of the game, and colourless is no colour at all \
         (CR 105.4): {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colours it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        2,
        "and the {{1}} ate one of the three green the Forests made"
    );
    assert_eq!(
        pool.total(),
        3,
        "three mana, and nothing else came with them"
    );
    assert!(
        is_tapped(&engine, land),
        "the {{T}} is the other half of that price"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );

    // ---- "{3}, {T}, Exile this land: return a historic card" -------------
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land), "the land is up again");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool emptied with the step that ended"
    );

    // A historic card in each graveyard. Sol Ring is an artifact (CR 700.6),
    // so both are legal *for the card's own filter* — and only mine is
    // `PlayerRel::You`.
    seed_graveyard(&mut engine, p0, 1);
    seed_graveyard(&mut engine, p1, 1);
    let mine = in_graveyard(&engine, p0, quiet_artifact()).expect("p0's graveyard was seeded");
    let theirs = in_graveyard(&engine, p1, quiet_artifact()).expect("p1's graveyard was seeded");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal
            .abilities
            .iter()
            .any(|(source, index)| *source == land && *index == 2),
        "{{3}} is not three: with an empty pool the historic line is absent \
         from the offer: {:?}",
        legal.abilities
    );

    tap_all_mana_but(&mut engine, p0, Some(abstergo_entertainment()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Forests in the pool, and the land kept back"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal
            .abilities
            .iter()
            .any(|(source, index)| *source == land && *index == 2),
        "with its {{3}} floating and its {{T}} spare, the third line is \
         offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, abstergo_entertainment(), 2);
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target historic card from your graveyard\" is a target choice, \
             got {:?}",
            engine.pending()
        );
    };
    assert_eq!(player, p0, "the activating seat names the target");
    assert_eq!(
        (min, max),
        (0, 1),
        "\"up to one target\": naming nothing is a legal answer (CR 115.6)"
    );
    assert!(
        player_options.is_empty(),
        "no part of this choice is a player target: {player_options:?}"
    );
    assert!(
        options.contains(&mine),
        "a historic card in *my* graveyard is on the menu: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "`PlayerRel::You` is what declines the Sol Ring across the table: {options:?}"
    );
    // CR 601.2c before CR 601.2h: while the question stands the land is still
    // on the battlefield, still untapped, and the {3} is still in the pool.
    assert!(
        on_battlefield(&engine, p0, abstergo_entertainment()).is_some(),
        "the target is named before the cost is paid"
    );
    assert!(
        !is_tapped(&engine, land),
        "and the {{T}} has not been paid yet either"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "nor has the {{3}} left the pool"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the historic card the question offered was chosen");

    assert!(
        on_battlefield(&engine, p0, abstergo_entertainment()).is_none(),
        "\"Exile Abstergo Entertainment\" is part of the cost, so the land is \
         gone before the ability resolves"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p0))
            .contains(&land),
        "and it is in exile rather than in anybody's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{3}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "returning a card is no mana ability, so the ability is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&mine),
        "\"return up to one target historic card from your graveyard to your \
         hand\": the very card that was named"
    );
    assert_eq!(
        engine.state().object(mine).map(|o| o.zone),
        Some(Zone::Hand),
        "and it is in a hand, not merely missing from a graveyard"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p1))
            .is_empty(),
        "\"then exile all graveyards\": the opponent's graveyard is emptied \
         too, which is the half a card reading only its controller would lose"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .is_empty(),
        "and mine is empty as well — the returned card left it for the hand"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p1))
            .contains(&theirs),
        "the opponent's historic card was exiled, and it is still the same \
         object it was in the graveyard"
    );
}
