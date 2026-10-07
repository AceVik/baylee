//! `cards/lands/cycling/coastal_peak.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Coastal Peak prints three lines: it is a `Land — Island Mountain` that
/// enters tapped, taps for {U} or {R}, and cycles for {2} (discard it, draw a
/// card). The board plays all three in the order the card itself forces them:
/// the land drop proves the tapped entry, the second copy in hand is what the
/// cycling cost discards — and a tapped land is no mana source, so the {U}/{R}
/// half only becomes readable one turn later, where the two colour options are
/// what tells "or" from "and". The two Forests are the paid {2} and nothing
/// else, which is why the floating pool is asserted on both sides of it.
#[test]
fn coastal_peak_enters_tapped_cycles_for_a_card_and_taps_for_either_colour() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(4242, island())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[coastal_peak(), coastal_peak()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The land drop, played for real rather than seeded: `starting_battlefield`
    // places with `Cause::Setup` and no replacement effect would look at it.
    let peak = play_land(&mut engine, p0, coastal_peak());
    assert!(
        entered_tapped(&engine, peak),
        "\"This land enters tapped\" (CR 614.1c)"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and a land that arrives tapped gives nothing on the turn it arrives"
    );

    // Cycling {2}: ability 1 on the card — ability 0 is the mana ability, which
    // a card in hand is not offered.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Forests pay the {{2}}; the tapped Coastal Peak is no source at all"
    );
    let library_before = library_size(&engine, p0);
    activate(&mut engine, p0, coastal_peak(), 1);
    assert!(
        matches!(drive_to_rest(&mut engine, p0), Rest::Reached),
        "the cycling cost is payable and its draw resolves"
    );
    assert!(
        in_graveyard(&engine, p0, coastal_peak()).is_some(),
        "\"Discard this card\" is the cost, so the cycled card is in the graveyard"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "and \"draw a card\" took one off the top — the only thing the {{2}} buys"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}} came out of the pool"
    );

    // One turn later, the untap step stands the played land back up.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    let peak = on_battlefield(&engine, p0, coastal_peak()).expect("the played land is still out");
    assert!(!is_tapped(&engine, peak), "the untap step stood it up");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    // `deeds` and not `legal.mana_abilities` alone: a land whose mana comes
    // from its basic land types is the CR 305.6 shortcut, and the same tap
    // printed by a card is an ordinary `(source, index)` entry (#159).
    let route = deeds(&legal, &[peak])
        .into_iter()
        .find_map(|(_, deed)| match deed {
            Deed::Mana | Deed::Ability(_) => Some(deed.action(peak)),
            Deed::Cast => None,
        })
        .expect("an untapped Coastal Peak offers the mana ability it prints");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing is floating before it is tapped, so the one mana below is its own"
    );
    engine.apply(p0, route).expect("the offer named the route");

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{U}} or {{R}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        player, p0,
        "the seat that tapped the land is the one that names the colour"
    );
    assert!(
        options.contains(&ManaColor::Blue) && options.contains(&ManaColor::Red),
        "the two colours the land prints: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "and colourless is no colour at all (CR 105.4)"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .expect("red was one of the two it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Red),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(pool.available(ManaColor::Blue), 0, "and not both of them");
    assert_eq!(pool.total(), 1, "one mana off one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
}
