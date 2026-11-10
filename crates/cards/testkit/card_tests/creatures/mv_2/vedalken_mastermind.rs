//! `cards/creatures/mv_2/vedalken_mastermind.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Vedalken Mastermind — {U}{U} Creature — Vedalken Wizard, 1/2:
/// "{U}, {T}: Return target permanent you control to its owner's hand."
///
/// "You control" is the whole card, so the *same* artifact stands under each
/// seat — a Sol Ring a side — and a menu that names one and declines the other
/// is what reads the filter rather than `Filter::Any`; the Mastermind itself is
/// on that menu too, because it too is a permanent you control. The {U} is read
/// as a real price in both directions: with an empty pool the line is absent
/// from the offer rather than refused (`can_afford` reads the pool and never the
/// untapped lands), and the pool is one shorter only once the target has been
/// answered — CR 601.2c before CR 601.2h, so while the question stands the
/// Mastermind is still untapped and the mana still floating.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn vedalken_mastermind_bounces_a_permanent_you_control_and_declines_the_opponents() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    // The Mastermind is seated rather than cast: its price is `{U}` *and* `{T}`,
    // and a creature that arrived this turn may not pay the tap symbol at all
    // (CR 302.6). A `SeatSpec` placement is the same permanent a cast a turn
    // earlier would have left, and nothing about the cast is what this reads.
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                quiet_artifact(),
                vedalken_mastermind(),
            ],
        )
        .battlefield(1, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches their own main phase"
    );

    let mastermind =
        on_battlefield(&engine, p0, vedalken_mastermind()).expect("the Mastermind is out");
    let mine = on_battlefield(&engine, p0, quiet_artifact()).expect("my Sol Ring is out");
    let theirs = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    assert_eq!(pt(&engine, mastermind), (1, 2), "the body the card prints");

    // An empty pool pays no {U}, and the whole price is the mana plus a tap
    // nobody has spent — so the line is missing from the offer for the mana
    // alone, which is the control the mana below is measured against.
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the seat with the Mastermind holds it");
    assert!(
        !legal.abilities.contains(&(mastermind, 0)),
        "with nothing floating the {{U}} cannot be paid, and an unpayable cost \
         is absent from the offer rather than refused: {:?}",
        legal.abilities
    );

    // Three Islands into the pool and the Sol Ring named as the thing kept
    // back: it is one of the permanents the bounce is about to choose between,
    // and a source tapped for mana has already changed for a reason of its own.
    tap_all_mana_but(&mut engine, p0, Some(quiet_artifact()));
    let floating = engine.state().players[0].mana_pool.total();
    assert_eq!(floating, 3, "three Islands, and the Sol Ring kept back");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(mastermind, 0)),
        "with the {{U}} floating, the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, vedalken_mastermind(), 0);
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target permanent you control\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat chooses");
    assert_eq!((min, max), (1, 1), "exactly one permanent");
    assert!(
        options.contains(&mine),
        "the Sol Ring beside it is a permanent this seat controls: {options:?}"
    );
    assert!(
        options.contains(&mastermind),
        "and so is the Mastermind itself — \"you control\" names no other \
         creature: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "\"you control\" is not \"any permanent\": the identical Sol Ring \
         across the table is not this seat's: {options:?}"
    );
    assert_eq!(
        options.len(),
        5,
        "the three Islands, the Sol Ring and the Mastermind — everything this \
         seat has and nothing else: {options:?}"
    );
    assert!(
        !is_tapped(&engine, mastermind),
        "CR 601.2c before CR 601.2h: the tap is not paid while the question stands"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        floating,
        "and neither is the {{U}}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the permanent the question offered was chosen");
    assert!(
        is_tapped(&engine, mastermind),
        "{{T}} is half the price and is paid as the activation completes"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        floating - 1,
        "and the {{U}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "returning a permanent is no mana ability, so it is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_none(),
        "the targeted permanent left the battlefield"
    );
    assert!(
        in_hand(&engine, p0, quiet_artifact()).is_some(),
        "\"to its owner's hand\": the Sol Ring is back in the hand of the seat \
         that owns it"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "the permanent the ability did not name never moved"
    );
    assert!(
        on_battlefield(&engine, p0, vedalken_mastermind()).is_some(),
        "an activated ability costs the creature nothing but its tap"
    );
}
