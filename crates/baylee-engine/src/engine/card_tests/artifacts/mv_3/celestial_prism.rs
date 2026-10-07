//! `cards/artifacts/mv_3/celestial_prism.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Celestial Prism prints one line — "{2}, {T}: Add one mana of any color."
/// — so the whole card is a price and a question. The price has two halves
/// the pool cannot see: the {2} comes out of a pool only three tapped
/// forests filled, and the {T} leaves the artifact tapped, so the mana it
/// makes and the mana it ate have to be read on the same activation. The
/// "any color" is read as the question it is — five options wide with no
/// colourless among them (CR 105.4) — and the black that lands afterwards has
/// no other source on the board: the forests make green and are spent by
/// then, so `{B}` can only have come off the Prism itself.
#[test]
fn celestial_prism_taps_and_two_mana_for_one_mana_of_any_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1013, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[celestial_prism()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Three Forests pay the {3}, and the {2} the ability charges is the other
    // half: it has to be paid out of the pool and not out of lands that are
    // still standing.
    cast_from_hand(&mut engine, p0, celestial_prism());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, celestial_prism()).is_some()
    });
    let prism = on_battlefield(&engine, p0, celestial_prism()).expect("the Prism resolved");
    assert!(
        !is_tapped(&engine, prism),
        "an artifact enters untapped, so its {{T}} is still there to pay"
    );

    // Read the offer where the engine reads it: `can_afford` looks at the
    // pool, so with nothing floating the {2} is unpayable and the line is not
    // there at all.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(prism, 0)),
        "{{2}} is not two, so the cost is unpayable and nothing is offered: {:?}",
        legal.abilities
    );

    // One turn round the table, because the three Forests that paid for the
    // Prism are still tapped: the {2} this ability charges has to come out
    // of lands that untapped, not out of the same three twice.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");

    // The Prism is named as the thing kept back: it prints its own mana
    // ability, so `tap_all_mana` would have spent the very permanent this
    // test activates by hand (#159).
    tap_all_mana_but(&mut engine, p0, Some(celestial_prism()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Forests in the pool and nothing off the Prism"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(prism, 0)),
        "with {{2}} floating the whole price is payable: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, celestial_prism(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("`any color` is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options.len(),
        5,
        "the five colors of the game, and colorless is no color at all \
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
        .expect("black was one of the colors it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the color that was named, and not a default"
    );
    assert_eq!(
        pool.total(),
        2,
        "three green arrived, the {{2}} ate two of them, and the Prism put \
         the black back: one green and one black"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "two of the three green paid the price and the third is still \
         floating — no source on this board but the Prism makes {{B}}"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, prism), "the Prism paid its own {{T}}");
}
