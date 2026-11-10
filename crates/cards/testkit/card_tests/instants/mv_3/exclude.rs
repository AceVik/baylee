//! `cards/instants/mv_3/exclude.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "a8c9f91a-b1e7-451d-b6db-ae865e2b853c"

/// Exclude prints two sentences — "Counter target creature spell" and "Draw a
/// card" — and a stack holding two spells proves both in one resolution. p0's
/// Llanowar Elves is the creature spell the counter is for, and the Dark Ritual
/// cast on top of it is the spell the printed filter has to decline, so the
/// target menu reads "creature" instead of assuming it. Countering is a move
/// between zones — the Elves ends in its owner's graveyard and never reaches
/// the battlefield — and the draw is read as the card that was on top of p1's
/// library arriving in p1's hand, which an Exclude that merely emptied the
/// stack could not produce.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn exclude_counters_a_creature_spell_and_draws_its_controller_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), swamp()])
        .hand(0, &[llanowar_elves(), dark_ritual()])
        .battlefield(1, &[island(), island(), island(), island()])
        .hand(1, &[exclude()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Two spells on the stack at once, and only one of them is a creature: p0
    // holds priority after the Elves (CR 601.2i) and puts the instant on top of
    // it with the black the Swamp pays for.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "two Forests and a Swamp, and no creature on the board to make more"
    );
    cast_with_floating(&mut engine, p0, llanowar_elves());
    cast_with_floating(&mut engine, p0, dark_ritual());
    let elves = on_stack(&engine, llanowar_elves()).expect("the Elves spell is on the stack");
    let ritual = on_stack(&engine, dark_ritual()).expect("the Ritual is on the stack beside it");
    assert_ne!(elves, ritual, "two spells, two objects");

    // p1's window. The mana is tapped before anything is claimed about the
    // offer, because `LegalActions` is filtered through `can_afford`, which
    // reads the pool and not the untapped Islands.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
            && on_stack(e, llanowar_elves()).is_some()
    });
    tap_all_mana(&mut engine, p1);
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        4,
        "four Islands, four blue"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let card = in_hand(&engine, p1, exclude()).expect("the Exclude is in hand");
    assert!(
        legal.castable.contains(&card),
        "{{2}}{{U}} is payable out of the pool, so the Exclude is castable: {:?}",
        legal.castable
    );

    // The top of p1's library, named before the draw, so the card that moves is
    // the one this test is about rather than any card that happens to be
    // missing afterwards.
    let library_before = engine.state().zones.list(ZoneLocation::Library(p1)).clone();
    let top = *library_before.last().expect("p1's library has a top card");
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p1)).len();

    cast_with_floating(&mut engine, p1, exclude());
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature spell\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p1, "the seat that cast the Exclude aims it");
    assert_eq!((min, max), (1, 1), "one target, and the spell asks once");
    assert!(
        options.contains(&elves),
        "the creature spell waiting on the stack is the target it is for: {options:?}"
    );
    assert!(
        !options.contains(&ritual),
        "\"creature spell\" is read, not skipped: the instant above it is a \
         spell and no creature: {options:?}"
    );
    assert_eq!(options.len(), 1, "and the Elves are the whole menu");

    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .expect("the creature spell was one of the options it enumerated");
    assert!(
        on_stack(&engine, dark_ritual()).is_some(),
        "the Ritual is untouched by the target that was named"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_stack(&engine, llanowar_elves()).is_none(),
        "the countered spell left the stack"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "and it never arrived: a countered spell resolves into a graveyard"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "the card is in its owner's graveyard, which is where a countered spell goes"
    );
    assert!(
        in_graveyard(&engine, p1, exclude()).is_some(),
        "and the Exclude followed it there once it had resolved"
    );
    assert_eq!(
        library_size(&engine, p1),
        library_before.len() - 1,
        "\"Draw a card\": one card left the top of the library"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p1))
            .contains(&top),
        "and the card that was on top is in hand, so the draw is not merely a \
         library that got shorter"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        hand_before,
        "one card out (the Exclude) and one card in (the draw)"
    );
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        1,
        "the {{2}}{{U}} came out of the pool"
    );
}
