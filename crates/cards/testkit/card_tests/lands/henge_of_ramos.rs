//! `cards/lands/henge_of_ramos.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn henge_of_ramos_taps_for_colorless_and_pays_two_for_any_color() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[henge_of_ramos()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let henge = play_land(&mut engine, p0, henge_of_ramos());
    assert!(
        !is_tapped(&engine, henge),
        "kein Entry-Modifier gedruckt, und der Land-Drop hat kein Mana gekostet"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the pool is empty before anything is tapped"
    );

    // The {2} is a real cost and the offer is read at the pool
    // (CR 601.2h): without floating mana, the second ability is not
    // available to choose, while the first — whose entire cost is its own
    // tap symbol — is.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "der Sitz hat eine ruhige Hauptphase, got {:?}",
            engine.pending()
        )
    };
    assert!(
        !legal.abilities.contains(&(henge, 1)),
        "{{2}} is not payable from an empty pool"
    );
    assert!(
        legal.abilities.contains(&(henge, 0)),
        "während {{T}} allein es ist: {:?}",
        legal.abilities
    );

    // The first half: {2}, {T} for a color of your choice. The two
    // Forests go first, and the Henge is the source that remains.
    tap_mana_except(&mut engine, p0, henge);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Forests, and the Henge is still standing"
    );
    assert!(!is_tapped(&engine, henge));

    activate(&mut engine, p0, henge_of_ramos(), 1);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("`any color` ist eine Frage, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat names the colour");
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        assert!(
            options.contains(&color),
            "\"any color\" schließt {color:?} ein: {options:?}"
        );
    }
    assert_eq!(
        options.len(),
        5,
        "the five colors of the game, and colorless is not one at all (CR 105.4): {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("Black was one of the offered colors");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the named color, and no default"
    );
    assert_eq!(
        pool.total(),
        1,
        "three mana in, two as {{2}} back out again"
    );
    assert!(is_tapped(&engine, henge), "the {{T}} in the cost tapped it");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability does not use the stack"
    );

    // The second half is the bare {T}: Add {C}, read on the next turn,
    // when the untap step has untapped the land again.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "der Beherrscher des Henge kommt wieder an die Reihe"
    );
    assert!(
        !is_tapped(&engine, henge),
        "der Enttappschritt hat es zurückgegeben"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the Pool was emptied with the step (CR 500.5)"
    );

    activate(&mut engine, p0, henge_of_ramos(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1, "{{T}}: Add {{C}}");
    assert_eq!(
        pool.total(),
        1,
        "ein Mana aus einem Tap — die zwei ungetappten Wälder daneben stehen \
         unberührt, sonst stünden hier drei"
    );
    assert!(is_tapped(&engine, henge), "its own tap was the whole price");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: auch diese Hälfte benutzt keinen Stack"
    );
}
