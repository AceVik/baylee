//! `cards/artifacts/mv_4/staff_of_zegon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Staff of Zegon — {4} artifact: "{3}, {T}: Target creature gets -2/-0 until
/// end of turn."
///
/// Both halves of the price land where a test can read them, and neither is a
/// label: seven Forests are exactly the {4} the cast costs plus the {3} the
/// ability then charges, so the pool reads three before the activation and
/// nothing after it. The target question is answered while the artifact is
/// still untapped and the mana still floating (CR 601.2c before CR 601.2h),
/// which is the order that tells a real cost from one paid on announcement,
/// and a printed 3/3 becoming a 1/3 is the only reading that applies the -2
/// without inventing a toughness the card never touches. The Elf across the
/// table is the other end of `Filter::CREATURE`: "target creature" names a
/// board, not a seat.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn staff_of_zegon_taps_and_three_mana_to_shrink_the_creature_it_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut board = vec![forest(); 7];
    board.push(katara_the_fearless());
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .hand(0, &[staff_of_zegon()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let host = on_battlefield(&engine, p0, katara_the_fearless()).expect("my creature is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert_eq!(pt(&engine, host), (3, 3), "a printed 3/3 before the staff");

    // `legal.castable` is filtered through `can_afford`, and that reads the
    // pool rather than the seven untapped Forests: with nothing floating the
    // {4} is unpayable, so the staff is not among the castable cards at all.
    let card = in_hand(&engine, p0, staff_of_zegon()).expect("the staff is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&card),
        "an empty pool pays no {{4}}, so the staff is not offered: {:?}",
        legal.castable
    );

    // Seven Forests and nothing else: the creature is named as the printing
    // kept back, because it is the permanent the ability is about to be aimed
    // at and a creature tapped for mana would read as a different board.
    tap_all_mana_but(&mut engine, p0, Some(katara_the_fearless()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        7,
        "seven tapped Forests, seven green, and the creature contributed nothing"
    );
    cast_with_floating(&mut engine, p0, staff_of_zegon());
    pass_until(&mut engine, stack_is_empty);
    let staff = on_battlefield(&engine, p0, staff_of_zegon()).expect("the staff resolved");
    assert!(!is_tapped(&engine, staff), "an artifact enters untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "the cast's {{4}} is spent and exactly the {{3}} the ability charges is left"
    );

    // `LegalActions::abilities` is filtered through `can_afford`, which reads
    // the pool and not the untapped lands — so the claim about the offer is
    // made with the mana already floating, where the engine reads it.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(staff, 0)),
        "with {{3}} in the pool the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, staff_of_zegon(), 0);
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
        options.contains(&host) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&staff),
        "the staff is an artifact and no creature: {options:?}"
    );
    // CR 601.2c names the target first and CR 601.2h pays afterwards, so both
    // prices are still unpaid while this question stands.
    assert!(
        !is_tapped(&engine, staff),
        "the {{T}} is the last step of the activation, not the first"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "and the {{3}} is still in the pool for the same reason"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the creature the question offered was chosen");

    assert!(is_tapped(&engine, staff), "{{T}} is paid by the artifact");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{3}} it charges came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "a pump is no mana ability, so the ability is waiting on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, host),
        (1, 3),
        "-2/-0 on the creature the staff named: power down by two, and a \
         toughness the card never touches left exactly where it was"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the pump reaches the creature it targeted and never across the table"
    );
    assert!(
        on_battlefield(&engine, p0, staff_of_zegon()).is_some(),
        "the price was a tap and no sacrifice, so the artifact is still standing"
    );
}
