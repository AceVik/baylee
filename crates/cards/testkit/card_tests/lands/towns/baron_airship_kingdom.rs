//! `cards/lands/towns/baron_airship_kingdom.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Baron, Airship Kingdom — Land — Town: "This land enters tapped" and
/// "{T}: Add {U} or {R}". Neither printed line is visible in the card file,
/// so both are played: the land arrives by a real `PlayLand` rather than a
/// `starting_battlefield` placement — which places a permanent without an
/// entry and would read untapped whatever the card says — and the tapped
/// status read straight afterwards is therefore the printed entry modifier.
/// The untap step is the control for it (the land comes back up on its
/// controller's next turn, which is what tells an entry that tapped it from a
/// permanent that never untaps), and that same turn is where the `{T}` line
/// the tapped state hid is finally offered. "Or" is the last reading: the
/// choice is exactly the two colours the card names, and naming one is not
/// making both.
#[test]
fn baron_airship_kingdom_enters_tapped_and_taps_for_blue_or_red() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[baron_airship_kingdom()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let baron = play_land(&mut engine, p0, baron_airship_kingdom());
    assert!(
        entered_tapped(&engine, baron),
        "\"This land enters tapped\" — and it was played, not placed"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "a land drop spends no mana, so the pool is empty while it lies tapped"
    );

    // A tapped permanent pays no {T}, and {T} is the whole price of the
    // printed mana line: on the turn it arrives the land does nothing.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.contains(&(baron, 0)),
        "a tapped land has no {{T}} to pay with, so the mana line is not even \
         offered: {:?}",
        legal.abilities
    );

    // Across the opponent's turn and back: the untap step is what stands the
    // land up again, so a permanent that stayed down here would be a rule and
    // not an entry.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, baron),
        "the untap step stood the land back up"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool emptied with the step that ended (CR 500.5)"
    );

    // The whole price of the ability is its own tap, so it is offered on an
    // empty pool — and with no other permanent on the board, whatever lands in
    // the pool afterwards came off this land.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(baron, 0)),
        "an untapped land is a paid {{T}}, so the line is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, baron_airship_kingdom(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{U}} or {{R}}\" is a question, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options.len(),
        2,
        "the two colours the card prints, and no third: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Blue) && options.contains(&ManaColor::Red),
        "both halves of `or` are on the menu: {options:?}"
    );
    assert!(
        !options.contains(&ManaColor::Colorless),
        "colourless is no colour at all (CR 105.4): {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("blue was one of the colours it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Red),
        0,
        "`or` is one mana of one colour: the other half of the menu was not \
         added beside it"
    );
    assert_eq!(
        pool.total(),
        1,
        "one mana off one tap, and nothing else on the board could have made it"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the seat holds priority again, got {:?}",
        engine.pending()
    );
    assert!(is_tapped(&engine, baron), "the land paid its own {{T}}");
}
