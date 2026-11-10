//! `cards/artifacts/mv_3/darksteel_ingot.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Darksteel Ingot — {3} artifact: "Indestructible" and "{T}: Add one mana of
/// any color."
///
/// Both halves are the engine's answer rather than the card's, so both are
/// played. The `{T}` asks for one of five *colors* — colorless is no color at
/// all (CR 105.4) — and the mana is in the pool with an empty stack, because a
/// mana ability resolves as it is activated (CR 605.3b); the black that lands
/// is the reading, since three tapped Forests make green and nothing else on
/// this board can make black. The keyword is played too: an opponent's
/// Vindicate names the Ingot and it goes nowhere, which is the one outcome
/// that tells `Indestructible` (CR 702.12b) from a card that merely prints the
/// word.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn darksteel_ingot_taps_for_a_color_of_its_controllers_choosing_and_survives_a_destroy() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    // Three Forests pay the {3} exactly, so the pool is empty once the Ingot
    // has landed; Plains, Swamp and Plains across the table pay a Vindicate.
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[darksteel_ingot()])
        .battlefield(1, &[plains(), swamp(), plains()])
        .hand(1, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, darksteel_ingot());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let ingot = on_battlefield(&engine, p0, darksteel_ingot()).expect("the Ingot resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "three Forests paid the {{3}} exactly, so the pool reads as empty"
    );
    assert!(
        keywords(&engine, ingot).contains(KeywordSet::INDESTRUCTIBLE),
        "the printed Indestructible reaches the permanent"
    );

    // Ability 0 is the printed "{T}: Add one mana of any color." Its whole
    // price is its own tap, so it is offered without a single mana floating.
    activate(&mut engine, p0, darksteel_ingot(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("\"any color\" is a question, got {:?}", engine.pending());
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
        pool.available(ManaColor::Green),
        0,
        "the three Forests were spent on the cast and make green anyway, so \
         nothing still standing on this board could have produced the black"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, ingot), "the Ingot paid its own {{T}}");

    // The other printed line, played rather than read: a destroy effect names
    // the Ingot and the Ingot stays where it is (CR 702.12b).
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, vindicate());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!("pass_until only stops on a target choice")
    };
    assert!(
        options.contains(&ingot),
        "\"target permanent\" names any permanent, the indestructible one \
         included: {options:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![ingot],
            },
        )
        .expect("the Ingot was one of the options it enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, darksteel_ingot()).is_some(),
        "\"effects that say destroy don't destroy this artifact\" — the \
         Vindicate resolved against it and it is still on the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, darksteel_ingot()).is_none(),
        "and it was not moved anywhere else either: nothing put it in a graveyard"
    );
    assert!(
        in_graveyard(&engine, p1, vindicate()).is_some(),
        "the Vindicate itself resolved rather than being countered or fizzling"
    );
}
