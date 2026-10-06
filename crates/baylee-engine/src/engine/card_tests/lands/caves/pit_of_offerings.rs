//! `cards/lands/caves/pit_of_offerings.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Pit of Offerings is a Cave that enters tapped, exiles up to three target
/// cards from any graveyard as it arrives, and taps for {C} — the three
/// clauses the card file says are built. Playing it is the only reading of the
/// first two: a permanent seeded through `starting_battlefield` is *placed*
/// rather than entering (CR 614.12), so a board built that way would show the
/// land upright on arrival and prove nothing about the printed sentence. Both
/// graveyards are read because the spec reaches every graveyard, and the one
/// card left behind is what separates the choice from a blanket exile; mana of
/// the exiled cards' colors is the `Coverage::Partial` gap and is not asked
/// for here.
#[test]
#[allow(clippy::too_many_lines)] // one land, played through every clause it prints
fn pit_of_offerings_enters_tapped_exiles_from_either_graveyard_and_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(404, forest())
        .hand(0, &[pit_of_offerings()])
        .start();
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Graveyards for the entry trigger to look at: two cards of mine and one
    // across the table, so the offer has to reach past my own.
    seed_graveyard(&mut engine, p0, 2);
    seed_graveyard(&mut engine, p1, 1);
    let mine: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(p0))
        .clone();
    let theirs: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(p1))
        .clone();
    assert_eq!((mine.len(), theirs.len()), (2, 1));

    let land = play_land(&mut engine, p0, pit_of_offerings());
    assert!(
        entered_tapped(&engine, land),
        "\"This land enters tapped\" — a placement would have skipped the \
         replacement effect and left it standing"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the land's controller is asked");
    assert_eq!((min, max), (0, 3), "\"up to three target cards\"");
    assert!(
        player_options.is_empty(),
        "cards in graveyards, not players: {player_options:?}"
    );
    assert_eq!(
        options.len(),
        3,
        "two cards in my graveyard and one in the opponent's: {options:?}"
    );
    for card in mine.iter().chain(theirs.iter()) {
        assert!(
            options.contains(card),
            "every graveyard is reachable, not only mine: {options:?}"
        );
    }

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine[0], theirs[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    for exiled in [mine[0], theirs[0]] {
        assert_eq!(
            engine.state().object(exiled).expect("still a card").zone,
            Zone::Exile,
            "the two cards that were named are exiled, wherever they came from"
        );
    }
    assert_eq!(
        engine.state().object(mine[1]).expect("still a card").zone,
        Zone::Graveyard,
        "and the card nobody named is where it was"
    );

    // The land arrived tapped, so its own {T} waits on the untap step the
    // opponent's turn sits in front of.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "and the Cave's controller takes another turn"
    );
    assert!(
        !is_tapped(&engine, land),
        "the untap step stood it back up, so entering tapped is a one-time \
         event and not a permanent state"
    );

    activate(&mut engine, p0, pit_of_offerings(), 1);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1, "{{T}}: Add {{C}}");
    assert_eq!(pool.total(), 1, "one mana, and nothing else came with it");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "and no color question was asked of it: {{C}} is the printed route, and \
         the exiled cards' colors are the gap, got {:?}",
        engine.pending()
    );
}
