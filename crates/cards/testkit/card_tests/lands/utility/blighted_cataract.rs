//! `cards/lands/utility/blighted_cataract.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Blighted Cataract prints two lines: "{T}: Add {C}." and "{5}{U}, {T},
/// Sacrifice this land: Draw two cards."
///
/// Every part of that second price is invisible in the card file, so the board
/// plays both halves on two turns: the land is played through the real land
/// drop and taps for one colourless on the turn it arrives, six Islands then
/// pay the {5}{U} one turn later while the Cataract is the single source kept
/// back from the tapping — and the offer is read before *and* after the mana is
/// floating, because `can_afford` reads the pool and not the untapped lands.
/// The {T} is read on the first turn, where the tapped Cataract is refused the
/// line with every mana it charges already floating; the sacrificed land and
/// the two drawn cards then land in two different zones, so no part of the
/// price can quietly have been skipped.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn blighted_cataract_taps_for_colorless_and_then_sells_itself_for_two_cards() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[island(), island(), island(), island(), island(), island()],
        )
        .hand(0, &[blighted_cataract()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The land arrives the way a land arrives: through the land drop. A
    // `starting_battlefield` placement would put a permanent on the table
    // without ever asking whether it may be played at all.
    let cataract = play_land(&mut engine, p0, blighted_cataract());
    assert!(
        !is_tapped(&engine, cataract),
        "a land with no enter modifier comes in untapped"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and playing it spends nothing"
    );

    // Ability 0: "{T}: Add {C}." A printed mana ability, so it is an ordinary
    // `(source, index)` entry whose whole price is its own tap symbol.
    activate(&mut engine, p0, blighted_cataract(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1, "{{T}}: Add {{C}}");
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(
        is_tapped(&engine, cataract),
        "the Cataract paid its own {{T}}"
    );
    assert!(
        !matches!(engine.pending(), Pending::ChooseColor { .. }),
        "`{{C}}` is fixed and not \"any colour\", so nothing is asked on the \
         way: {:?}",
        engine.pending()
    );

    // The {T} in the second price is read here, while the Cataract is the
    // one thing standing tapped: the six Islands put {5}{U} and more into the
    // pool beside its {C}, so every mana the line charges is floating, and
    // the one part of the price left unpayable is the tap it already spent.
    // After the payment there is nothing left to read it on — the same
    // payment sacrifices the land, and a card in a graveyard is a new object
    // that carries no tapped status (CR 400.7).
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        7,
        "six blue off the Islands beside the one colourless"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(cataract, 1)),
        "{{T}} is part of the price, and a tapped Cataract cannot pay it even \
         with the {{5}}{{U}} floating: {:?}",
        legal.abilities
    );

    // A turn later, because the {5}{U} line needs the same {T} the line above
    // just spent; the untap step (CR 502.3) is what hands it back.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, cataract),
        "the untap step stood the Cataract back up"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool emptied with the step that ended (CR 500.5)"
    );

    // `legal.abilities` is filtered through `can_afford`, and that reads the
    // pool rather than the six untapped Islands: with nothing floating the
    // {5}{U} is unpayable and the line is not there at all.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(cataract, 1)),
        "an empty pool pays no {{5}}{{U}}, so the sacrifice line is not \
         offered: {:?}",
        legal.abilities
    );

    // The Cataract is named as the one source kept back: it prints its own
    // `{T}: Add {C}`, and `tap_all_mana` would have spent the very tap the
    // sacrifice line needs (#159).
    tap_all_mana_but(&mut engine, p0, Some(blighted_cataract()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six Islands tapped for six blue, and the Cataract still standing untapped"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(cataract, 1)),
        "with six mana floating the whole price is payable: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // Ability 1: "{5}{U}, {T}, Sacrifice this land: Draw two cards." CR 601.2h
    // pays all three parts at once, as the ability is activated.
    activate(&mut engine, p0, blighted_cataract(), 1);
    assert!(
        in_graveyard(&engine, p0, blighted_cataract()).is_some(),
        "\"Sacrifice this land\" is the other half of the cost"
    );
    assert!(
        on_battlefield(&engine, p0, blighted_cataract()).is_none(),
        "and a sacrificed permanent leaves the battlefield"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{5}}{{U}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "drawing cards is no mana ability, so the ability is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        library_size(&engine, p0),
        library_before - 2,
        "\"Draw two cards\": two off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 2,
        "and they are in hand, so an emptied library would not satisfy the count above"
    );
}
