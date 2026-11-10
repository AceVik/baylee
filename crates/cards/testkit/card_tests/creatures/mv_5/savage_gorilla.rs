//! `cards/creatures/mv_5/savage_gorilla.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "6c876a57-c567-4d26-8ab6-a9c051e83ace"

// Phyrexian Fleshgorger — a 7/5 whose printed keywords are all an
// unimplemented stub, so seated on a battlefield it is a body and no rule at
// all. That is what the shrink below needs: a creature whose power and
// toughness both have room to lose three.
// oracle_id = "d3a5a830-cd14-49da-9412-c50049c74c92"

/// Savage Gorilla — {4}{G} 3/3 Ape: "{U}{B}, {T}, Sacrifice this creature:
/// Target creature gets -3/-3 until end of turn. Draw a card."
///
/// Every part of that line is the engine's answer rather than the card's, so one
/// board reads all of them. The Gorilla is seated instead of cast, because its
/// own price carries the tap symbol and a creature that arrived this turn cannot
/// pay one (CR 302.6), and an Island plus a Swamp put exactly {U}{B} in the pool
/// — the same pool is then read empty, which is what makes the printed cost a
/// payment and not a label. The 7/5 across the table becomes a 4/2, both numbers
/// three lower, which neither a power-only pump nor a single -1/-1 could
/// produce, and the Elf beside the Gorilla stays a printed 1/1 so the effect is
/// shown to target a creature and not a side of the table.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn savage_gorilla_sacrifices_itself_to_shrink_a_creature_and_draw_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[savage_gorilla(), island(), swamp(), quiet_creature()])
        .battlefield(1, &[a_seven_five()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let gorilla = on_battlefield(&engine, p0, savage_gorilla()).expect("the Gorilla is seated");
    let elf = on_battlefield(&engine, p0, quiet_creature()).expect("the Elf is out");
    let victim = on_battlefield(&engine, p1, a_seven_five()).expect("their Wurm is out");
    assert_eq!(pt(&engine, victim), (7, 5), "a 7/5 before the -3/-3");
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "and a printed 1/1 beside the Gorilla"
    );

    // `LegalActions::abilities` is filtered through `can_afford`, which reads the
    // pool and not the untapped lands: with nothing floating the {U}{B} is
    // unpayable and the line is not offered at all.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(gorilla, 0)),
        "an empty pool pays no {{U}}{{B}}, so nothing is offered: {:?}",
        legal.abilities
    );

    // The two lands are tapped and the Elf is named as the printing kept back —
    // it is the second creature the target question has to offer, and a mana
    // creature tapped for the cost would put its own {G} in the pool that is
    // about to be read as exactly the printed price.
    tap_all_mana_but(&mut engine, p0, Some(quiet_creature()));
    {
        let pool = &engine.state().players[0].mana_pool;
        assert_eq!(pool.available(ManaColor::Blue), 1, "the Island's blue");
        assert_eq!(pool.available(ManaColor::Black), 1, "the Swamp's black");
        assert_eq!(
            pool.total(),
            2,
            "and nothing else: exactly the printed price"
        );
    }

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(gorilla, 0)),
        "with {{U}}{{B}} floating the Gorilla's one line is offered: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(&mut engine, p0, savage_gorilla(), 0);
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that aims it");
    assert_eq!((min, max), (1, 1), "one target, and the ability asks once");
    assert!(
        options.contains(&victim) && options.contains(&elf),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    // CR 601.2c names the target before CR 601.2h pays for it, so the Gorilla is
    // still a permanent and the mana is still in the pool while this stands.
    assert!(
        on_battlefield(&engine, p0, savage_gorilla()).is_some(),
        "the sacrifice is a cost and is paid after the target is chosen"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "and the {{U}}{{B}} is still floating for the same reason"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![victim],
            },
        )
        .expect("the creature across the table was one of the options");

    assert!(
        on_battlefield(&engine, p0, savage_gorilla()).is_none(),
        "\"Sacrifice this creature\" is part of the price, so it left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, savage_gorilla()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{U}}{{B}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "shrinking a creature is no mana ability, so the ability is on the stack"
    );

    pay_life_ward(&mut engine, p0, 7);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, victim),
        (4, 2),
        "\"gets -3/-3 until end of turn\": both numbers three lower, which a \
         power-only pump or a single -1/-1 could not produce"
    );
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "the creature the ability did not name is untouched, so the effect \
         targets one creature and does not sweep a board"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card left the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "and it reached the hand, so an emptied library would not satisfy the count above"
    );

    // "until end of turn": the Wurm is a 7/5 again once the turn it was shrunk
    // in is behind the table, so the -3/-3 was a duration and not a new body.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, victim),
        (7, 5),
        "the shrink lasted the turn it was made in and no longer"
    );
}
