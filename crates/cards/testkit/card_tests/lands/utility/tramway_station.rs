//! `cards/lands/utility/tramway_station.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Tramway Station is a land that enters tapped, taps for {B} or {R}, and can
/// be sacrificed along with {2}{B}{R} and its own tap to draw a card. All three
/// printed lines are played in one game: the entry is read as the tap ability
/// being absent on the turn the land arrives and on offer the turn after, the
/// mana ability as the two-colour question it asks, and the sacrifice as the
/// card leaving the battlefield for its owner's graveyard while the library
/// shrinks by exactly one. The entry has to be a real land drop — a permanent
/// seeded onto the battlefield by the harness is placed rather than entered, and
/// no replacement effect looks at it.
#[test]
#[allow(clippy::too_many_lines)]
fn tramway_station_enters_tapped_taps_for_black_or_red_and_trades_itself_for_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), mountain(), mountain()])
        .hand(0, &[tramway_station()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let land = play_land(&mut engine, p0, tramway_station());
    assert!(
        on_battlefield(&engine, p0, tramway_station()).is_some(),
        "the land drop put the permanent on the table"
    );
    assert!(
        entered_tapped(&engine, land),
        "the printed entry modifier is what puts it down tapped"
    );

    // A tapped land has no {T} to pay with, and the mana ability's *whole*
    // price is that tap — so the line is not on offer at all this turn. The
    // draw line is left out of this reading on purpose: it wants {2}{B}{R}
    // besides the tap, and `can_afford` reads the pool, so its absence here
    // would prove nothing.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "the land drop leaves the seat holding priority, got {:?}",
            engine.pending()
        )
    };
    assert!(
        !legal.abilities.contains(&(land, 0)),
        "a tapped Tramway Station has no {{T}} left to pay with: {:?}",
        legal.abilities
    );

    // The untap step is the only thing that gives the tap back, so the reading
    // above is a claim about this turn rather than about a board with no land.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");
    assert!(!is_tapped(&engine, land), "the untap step stood it back up");

    activate(&mut engine, p0, tramway_station(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{B}} or {{R}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options,
        vec![ManaColor::Black, ManaColor::Red],
        "the two colours the card prints, and no third"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colours it offered");

    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    let tapped: Vec<ObjectId> = lands_of(&engine, p0)
        .into_iter()
        .filter(|id| is_tapped(&engine, *id))
        .collect();
    assert_eq!(
        tapped,
        vec![land],
        "the Station paid its own {{T}} and no other land moved"
    );

    // The sacrifice line charges the same tap the mana ability just spent, so
    // it needs a turn of its own as well; the four other lands then cover
    // {2}{B}{R} between them while the Station is kept back for its {T}.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes a third turn");
    assert!(
        !is_tapped(&engine, land),
        "and the untap step gives it back"
    );
    tap_all_mana_but(&mut engine, p0, Some(tramway_station()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "two Swamps and two Mountains, and nothing off the Station"
    );

    // `can_afford` reads the pool, so the claim about the offer is only made
    // once the whole price is already floating.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(land, 1)),
        "with {{2}}{{B}}{{R}} in the pool the sacrifice line is offered: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(&mut engine, p0, tramway_station(), 1);
    assert!(
        on_battlefield(&engine, p0, tramway_station()).is_none(),
        "sacrificing the land is part of the cost, so it left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, tramway_station()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}}{{B}}{{R}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "drawing a card is no mana ability, so the ability uses the stack"
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
        "and it is in hand, so an emptied library would not satisfy the count above"
    );
}
