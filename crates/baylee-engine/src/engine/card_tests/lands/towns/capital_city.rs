//! `cards/lands/towns/capital_city.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "2c96ab90-155b-4bf4-acc9-65a2f0cd3189"

/// Capital City prints three lines and all three are the engine's: "{T}: Add
/// {C}", "{1}, {T}: Add one mana of any color", and cycling {2}. The two mana
/// lines share one tap symbol at two different prices, so the board carries two
/// copies — the first pays its `{T}` for the colourless line and that
/// colourless then pays the `{1}` of the second, which is the card's own halves
/// closing on each other — while the copy left in hand is cycled away for its
/// `{2}`. Every claim is read off a pool that is empty beforehand, so "one
/// mana" is one activation and not a leftover.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn capital_city_taps_for_colorless_pays_a_mana_for_any_color_and_cycles() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), capital_city()])
        .hand(0, &[capital_city(), capital_city()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let city_on_board =
        on_battlefield(&engine, p0, capital_city()).expect("the seated City is on the battlefield");
    let city_played = play_land(&mut engine, p0, capital_city());

    // Both printed mana lines are priced off the tap symbol, but only the
    // colourless one is priced off the tap symbol *alone*: `legal.abilities`
    // runs `can_afford` over the mana pool, so an empty pool offers the first
    // line and withholds the second.
    let Pending::Priority { player, legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending());
    };
    assert_eq!(player, p0, "and it is the seat that played the land");
    assert!(
        legal.abilities.contains(&(city_played, 0)),
        "{{T}}: Add {{C}} is the whole price of one tap: {:?}",
        legal.abilities
    );
    assert!(
        !legal.abilities.contains(&(city_played, 1)),
        "and {{1}}, {{T}} is not payable while nothing floats: {:?}",
        legal.abilities
    );

    // Two Forests into the pool, with both Cities named as the printing kept
    // back: their `{T}` is what every line below spends, and `tap_all_mana`
    // would have taken it (#159).
    tap_all_mana_but(&mut engine, p0, Some(capital_city()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two tapped Forests, two green, and nothing off either City"
    );

    // Cycling {2}, on the copy that is still in hand — an activation of a
    // *card* rather than of a permanent, so the offer carries it beside the two
    // the battlefield is holding. The discard is the cost and the draw is the
    // resolution, so the hand ends the size it started.
    let cycled = in_hand(&engine, p0, capital_city()).expect("a second copy is still in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(cycled, 2)),
        "{{2}}, Discard this card: Draw a card is offered while the two green \
         float: {:?}",
        legal.abilities
    );
    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: cycled,
                ability_index: 2,
            },
        )
        .expect("the two green floating pay the cycling cost");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}} came out of the pool"
    );
    assert!(
        in_graveyard(&engine, p0, capital_city()).is_some(),
        "the discarded card is in its owner's graveyard, which is where a \
         discarded card goes and not where an exile would"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card left the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "the discard cost one card and the draw replaced it — a count that had \
         fallen by one would be a discard with no draw behind it"
    );

    // Ability 0: "{{T}}: Add {{C}}." Its whole price is the tap symbol, so it
    // is offered on the empty pool the cycling left, and `{C}` is fixed rather
    // than chosen: nothing is asked on the way.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(city_on_board, 0)),
        "an untapped City is a paid {{T}}, so the line is offered: {:?}",
        legal.abilities
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: city_on_board,
                ability_index: 0,
            },
        )
        .expect("the whole price of the line is its own tap");
    assert!(
        !matches!(engine.pending(), Pending::ChooseColor { .. }),
        "`{{C}}` is no colour of the game (CR 105.4), so nothing is named: {:?}",
        engine.pending()
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );
    assert!(
        is_tapped(&engine, city_on_board),
        "the City paid its own {{T}}"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        1,
        "\"{{T}}: Add {{C}}\" — one, and the mana no colour can name"
    );
    assert_eq!(
        pool.total(),
        1,
        "one mana, off one tap, with the Forests' green already spent on the cycling"
    );

    // Ability 1: "{{1}}, {{T}}: Add one mana of any color", on the copy that
    // was played, paid with the colourless the other copy just made.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(city_played, 1)),
        "with the {{C}} floating the coloured line is payable: {:?}",
        legal.abilities
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: city_played,
                ability_index: 1,
            },
        )
        .expect("the colourless in the pool is the {1} it charges");
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("\"any color\" is a question, got {:?}", engine.pending());
    };
    assert_eq!(player, p0, "the seat that paid the {{1}} names the colour");
    assert_eq!(
        options.len(),
        5,
        "the five colours of the game, and colourless is no colour at all \
         (CR 105.4): {options:?}"
    );
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        assert!(
            options.contains(&color),
            "\"any color\" includes {color:?}: {options:?}"
        );
    }
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colours it offered");

    assert!(
        is_tapped(&engine, city_played),
        "and the copy that was played paid its own {{T}}"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Colorless),
        0,
        "the {{C}} the other City made was the {{1}} this line charges"
    );
    assert_eq!(
        pool.total(),
        1,
        "one mana, off one activation and the {{C}} of the City beside it"
    );
}
