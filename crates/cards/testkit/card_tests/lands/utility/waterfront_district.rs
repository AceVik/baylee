//! `cards/lands/utility/waterfront_district.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Waterfront District prints three sentences: it enters tapped, "{T}: Add {U}
/// or {B}", and "{2}{U}{B}, {T}, Sacrifice this land: Draw a card." All three
/// are read off one game, and each needs its own evidence — the land is
/// *played* rather than placed, because a placement is no entry and a tapped
/// permanent offers neither of its lines in the turn it arrives; the colour
/// question is the printed pair and no wider; and the last line is only
/// claimed once the four mana are really in the pool, since `legal.abilities`
/// is filtered through `can_afford` and that reads the pool rather than the
/// untapped lands.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn waterfront_district_enters_tapped_chooses_blue_or_black_and_draws_for_its_own_sacrifice() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), swamp()])
        .hand(0, &[waterfront_district()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Played and not placed: `starting_battlefield` moves a card in with
    // `Cause::Setup`, which no replacement effect looks at, so only a real
    // land drop exercises the printed entry.
    let land = play_land(&mut engine, p0, waterfront_district());
    assert!(entered_tapped(&engine, land), "\"This land enters tapped\"");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == land),
        "a land that entered tapped has no {{T}} to pay with, so neither \
         printed line is offered at all: {:?}",
        legal.abilities
    );

    // A whole turn cycle each way: the untap step is what stands the land up.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land), "the untap step stood it back up");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool emptied with the step that ended (CR 500.5)"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(land, 0)),
        "{{T}}: Add {{U}} or {{B}} costs its own tap and nothing else, so it is \
         offered on an empty pool: {:?}",
        legal.abilities
    );
    assert!(
        !legal.abilities.contains(&(land, 1)),
        "{{2}}{{U}}{{B}} is not payable out of an empty pool, and `can_afford` \
         reads the pool rather than the untapped lands: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, waterfront_district(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{U}} or {{B}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options,
        vec![ManaColor::Blue, ManaColor::Black],
        "the two colours the card prints, and no third"
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
        pool.available(ManaColor::Blue),
        0,
        "`or` is one colour: the other half of the menu was not added beside it"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");

    // The last printed line, in its own turn: it sacrifices the land itself,
    // so the tap the mana ability spent above has to come back first.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land), "the untap step stood it back up");

    // The District is named as the source kept back: its own `{T}: Add …` is a
    // printed mana ability, so `tap_all_mana` would have spent the very
    // permanent this activation needs to tap.
    tap_all_mana_but(&mut engine, p0, Some(waterfront_district()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "three Islands and a Swamp tapped for four, and the District kept standing"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(land, 1)),
        "with {{2}}{{U}}{{B}} in the pool the last line is offered: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(&mut engine, p0, waterfront_district(), 1);
    assert!(
        in_graveyard(&engine, p0, waterfront_district()).is_some(),
        "\"Sacrifice this land\" is part of the price, so the card is in its \
         owner's graveyard and not merely gone"
    );
    assert!(
        on_battlefield(&engine, p0, waterfront_district()).is_none(),
        "and it has left the battlefield"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}}{{U}}{{B}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "drawing a card is no mana ability, so the ability is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card left the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "and it reached the hand, so an emptied library would not satisfy the \
         count above"
    );
}
