//! `cards/lands/utility/foggy_bottom_swamp.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Foggy Bottom Swamp prints three lines: it enters tapped, it taps for {B} or
/// {G}, and for {4} plus its own tap *and its own body* it draws a card.
///
/// The entry is a real land drop rather than a placement, because
/// `starting_battlefield` puts a permanent down without an entry and the
/// untapped land a placement leaves is exactly the board that cannot show the
/// printed clause. The other two lines want the land standing, which is a turn
/// away (CR 502.3), so the test walks the cycle: the colour line is read as the
/// question it is — two options, and none of the other three colours — and the
/// draw line is claimed with its {4} already floating, because `can_afford`
/// reads the pool and not the five untapped Forests.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn foggy_bottom_swamp_enters_tapped_taps_for_two_colours_and_sells_itself_for_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 5])
        .hand(0, &[foggy_bottom_swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, foggy_bottom_swamp());
    assert!(
        entered_tapped(&engine, land),
        "\"This land enters tapped\" — read off a real entry, not a placement"
    );

    // Tapped is not cosmetic: with the {{T}} already spent there is nothing to
    // pay either printed ability with, so neither is offered at all.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == land),
        "a land that just entered tapped has no {{T}} to pay with: {:?}",
        legal.abilities
    );

    // One turn cycle. The land's own mana line costs nothing but its {{T}}, so
    // it is offered on an empty pool and asks the two colours the card prints.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land), "the untap step stood it back up");

    activate(&mut engine, p0, foggy_bottom_swamp(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{B}} or {{G}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options.len(),
        2,
        "the two colours the card prints, and no third: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Black) && options.contains(&ManaColor::Green),
        "both halves of the printed choice are offered: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .expect("green was one of the colours it offered");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "the colour that was named"
    );
    assert_eq!(
        pool.available(ManaColor::Black),
        0,
        "\"or\" is one colour: the other half of the choice was not added beside it"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");

    // The tap is half of the draw line's price, so a tapped land offers it to
    // no pool at all — the five Forests are tapped here to put the {{4}} in
    // that pool before the claim is made.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "the green the land made plus five more off the Forests"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(land, 1)),
        "\"{{4}}, {{T}}, Sacrifice this land: Draw a card\" — the four are \
         floating and the tap is not, so the line is absent: {:?}",
        legal.abilities
    );

    // A second turn cycle for the line itself, and the offer is claimed only
    // once its generic half is really in the pool: an empty pool would leave it
    // absent for a reason that has nothing to do with the printed cost.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, land),
        "the untap step stood it up again"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(land, 0)),
        "its own {{T}} is the whole price of the mana line, so that one is \
         offered on an empty pool: {:?}",
        legal.abilities
    );
    assert!(
        !legal.abilities.contains(&(land, 1)),
        "{{4}} is not four with nothing floating: {:?}",
        legal.abilities
    );

    tap_all_mana_but(&mut engine, p0, Some(foggy_bottom_swamp()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five tapped Forests, and the land itself kept back to pay its own price"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(land, 1)),
        "with {{4}} in the pool the draw line is offered: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(&mut engine, p0, foggy_bottom_swamp(), 1);
    assert!(
        on_battlefield(&engine, p0, foggy_bottom_swamp()).is_none(),
        "the sacrifice is part of the price and is paid on announcement \
         (CR 601.2h)"
    );
    assert!(
        in_graveyard(&engine, p0, foggy_bottom_swamp()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{4}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "drawing a card is no mana ability, so the ability is waiting on the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "and it reached the hand, so an emptied library would not satisfy the \
         count above"
    );
}
