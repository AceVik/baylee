//! `cards/lands/utility/omashu_city.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Omashu City prints three lines — "This land enters tapped", "{T}: Add {R}
/// or {G}", and "{4}, {T}, Sacrifice this land: Draw a card" — and one game
/// reads all three, because each line is the other's control. The land is
/// played as the land drop, so the tapped entry is a real entry rather than the
/// harness' placement, and the tapped branch is what withholds the mana ability
/// on the turn it arrives; the untap step then hands the `{T}` back, and the
/// ability asks for one of exactly two colours while every other source on the
/// board is still untapped, so the single red in the pool can only be its own.
/// The `{4}` line is claimed only with the four Mountains really floating —
/// `can_afford` reads the pool and not the untapped lands — and its sacrifice
/// is read off the graveyard rather than merely off the battlefield.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end over three turns
fn omashu_city_enters_tapped_taps_for_red_or_green_and_eats_itself_for_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .hand(0, &[omashu_city()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // "This land enters tapped", played as the land drop: `starting_battlefield`
    // places a permanent without an entry, so a board built that way arrives
    // untapped whatever the card says (CR 614.1c reads the entry).
    let city = play_land(&mut engine, p0, omashu_city());
    assert!(entered_tapped(&engine, city), "\"This land enters tapped\"");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(city, 0)),
        "the {{T}} is already spent by the entry, so the mana ability is not \
         offered on the turn the land arrives: {:?}",
        legal.abilities
    );

    // A turn later: the untap step has stood the City back up and the pool
    // emptied with the step that ended (CR 500.5).
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, city),
        "the untap step stood the City back up"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool emptied with the step that ended"
    );

    // Ability 0 is the printed "{T}: Add {R} or {G}". A mana ability a card
    // prints has an index to name, so it is an ordinary entry in `abilities`
    // and never the CR 305.6 shortcut a basic land uses.
    activate(&mut engine, p0, omashu_city(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{R}} or {{G}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options,
        vec![ManaColor::Red, ManaColor::Green],
        "the two colours the card prints, and no third"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .expect("red was one of the colours it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Red),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "\"or\" is one mana of one colour: both halves would have made this two"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(is_tapped(&engine, city), "the City paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(
        all_on_battlefield(&engine, p0, mountain())
            .iter()
            .all(|id| !is_tapped(&engine, *id)),
        "and no land was tapped for it, so the red on the pool is the City's own \
         and not a Mountain that happened to pay"
    );

    // Ability 1 is "{4}, {T}, Sacrifice this land: Draw a card." — untapped
    // again, with the four Mountains really in the pool, because the offer is
    // filtered through `can_afford` and that reads the pool, not the lands.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, city),
        "the untap step stood it back up again"
    );
    tap_all_mana_but(&mut engine, p0, Some(omashu_city()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Mountains tapped, and the City kept back: its whole price is its \
         own {{T}}, so `tap_all_mana` would have spent the land this test \
         sacrifices by hand (#159)"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(city, 1)),
        "with {{4}} floating the whole price is payable: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(&mut engine, p0, omashu_city(), 1);
    assert!(
        on_battlefield(&engine, p0, omashu_city()).is_none(),
        "\"Sacrifice this land\" is part of the price and is paid as the ability \
         is announced (CR 601.2h)"
    );
    assert!(
        in_graveyard(&engine, p0, omashu_city()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{4}} it charges came out of the pool"
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
        "and it reached the hand, so an emptied library would not satisfy the count above"
    );
}
