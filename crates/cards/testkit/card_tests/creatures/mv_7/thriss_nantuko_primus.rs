//! `cards/creatures/mv_7/thriss_nantuko_primus.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Thriss, Nantuko Primus is a 5/5 legendary Insect Druid for {5}{G}{G}
/// whose whole rules text is "{G}, {T}: Target creature gets +5/+5 until end
/// of turn." The board is eight Forests and one Llanowar Elf of each seat, so
/// everything the card file cannot answer is legible at once: the {G} is a
/// real price — read against an empty pool the line is not offered at all,
/// and it is the moment the eight Forests are tapped — and "target creature"
/// reaches the Elf across the table and Thriss himself, while only the Elf
/// the pump names reads (6, 6). The other printed word is the duration, so a
/// turn is walked to show the +5/+5 gone and the creature still standing.
#[test]
#[allow(clippy::too_many_lines)]
fn thriss_nantuko_primus_pays_a_green_to_pump_the_creature_it_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    // Eight Forests: {5}{G}{G} for Thriss, and the {G} his ability charges on
    // a later turn. The Elf is the creature the pump is measured on and is
    // kept out of the mana, so no count below has a creature in it.
    let mut field = vec![forest(); 8];
    field.push(llanowar_elves());
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &field)
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[thriss_nantuko_primus()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The cast is a real payment out of the pool, and the Elf is named as the
    // one source kept back — it is the permanent the pump is about.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        8,
        "eight Forests tapped, and the Elf kept standing"
    );
    cast_with_floating(&mut engine, p0, thriss_nantuko_primus());
    pass_until(&mut engine, stack_is_empty);
    let thriss = on_battlefield(&engine, p0, thriss_nantuko_primus()).expect("Thriss resolved");
    assert_eq!(pt(&engine, thriss), (5, 5), "the body the card prints");

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");

    // "{G}, {T}" carries the tap symbol, so CR 302.6 withholds the ability
    // until Thriss has been under his controller's control since their most
    // recent turn began; one turn cycle is the cheapest way to say that.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    // `can_afford` reads the pool and not the untapped lands, so the empty
    // pool left by the step change is the reading that says the {G} is real.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.contains(&(thriss, 0)),
        "an empty pool pays no {{G}}, so the line is not offered at all: {:?}",
        legal.abilities
    );

    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        8,
        "the eight Forests again, and the Elf of mine still standing"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(thriss, 0)),
        "with eight green floating the whole price is payable: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, thriss_nantuko_primus(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat aims it");
    assert!(
        options.contains(&mine) && options.contains(&theirs) && options.contains(&thriss),
        "\"target creature\" is any creature, on either side of the table and \
         the source itself: {options:?}"
    );

    // CR 601.2c before CR 601.2h: while the question stands, Thriss is still
    // standing and the {G} is still in the pool.
    assert!(!is_tapped(&engine, thriss), "nothing has tapped him yet");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        8,
        "and nothing has been spent yet"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the Elf was one of the options the question enumerated");

    assert!(
        is_tapped(&engine, thriss),
        "{{T}} is the other half of the cost"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        7,
        "and the {{G}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "a pump is no mana ability, so the ability is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, mine),
        (6, 6),
        "+5/+5 on the creature Thriss named"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the effect targets, it does not sweep the table"
    );
    assert_eq!(
        pt(&engine, thriss),
        (5, 5),
        "and the source, which was on the menu too, is untouched"
    );

    // "until end of turn": the Elf is still there a turn later and the pump is
    // not — a static grant would still be on it here.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "the grant lasted the turn it was made in and no longer"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "and the creature is still standing, so the pump expired rather than \
         the creature leaving"
    );
}
