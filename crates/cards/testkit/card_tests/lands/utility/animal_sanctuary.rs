//! `cards/lands/utility/animal_sanctuary.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Animal Sanctuary prints two lines: "{T}: Add {C}" and "{2}, {T}: Put a
/// +1/+1 counter on target Bird, Cat, Dog, Goat, Ox, or Snake."
///
/// The second sentence names no controller, so the subtype filter is the whole
/// card and the board has to carry both halves of it: a Bird on each side of
/// the table and an Elf that is none of the six subtypes. The {2} is a real
/// price — the mana is pooled before anything is claimed about the offer,
/// because `can_afford` reads the pool and not the untapped lands — and
/// CR 601.2c before CR 601.2h is why the Sanctuary is still standing and the
/// mana still floating while the target question is open.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn animal_sanctuary_charges_two_and_its_own_tap_for_a_counter_on_a_bird() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                animal_sanctuary(),
                forest(),
                forest(),
                baleful_strix(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[baleful_strix()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sanctuary = on_battlefield(&engine, p0, animal_sanctuary()).expect("the Sanctuary is out");
    let mine = on_battlefield(&engine, p0, baleful_strix()).expect("my Bird is out");
    let theirs = on_battlefield(&engine, p1, baleful_strix()).expect("their Bird is out");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");
    let before = pt(&engine, mine);
    let before_theirs = pt(&engine, theirs);

    // {2} out of the pool, because `can_afford` reads the pool rather than the
    // untapped lands. The Sanctuary is named as the printing kept back: its own
    // printed "{T}: Add {C}" is a route `tap_all_mana` would press (#159), and
    // the line this test presses is the other one.
    tap_all_mana_but(&mut engine, p0, Some(animal_sanctuary()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "two Forests and one Elf: every source on the board but the Sanctuary"
    );
    assert!(
        !is_tapped(&engine, sanctuary),
        "and the Sanctuary is still standing, so its {{T}} is unspent"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(sanctuary, 1)),
        "ability 0 is the printed \"{{T}}: Add {{C}}\" and ability 1 is the \
         counter line, offered now that its {{2}} is in the pool: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, animal_sanctuary(), 1);
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target Bird, Cat, Dog, Goat, Ox, or Snake\" is a target choice, \
             got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that aims it");
    assert_eq!((min, max), (1, 1), "one target, and the ability asks once");
    assert!(
        options.contains(&mine),
        "a Bird this seat controls is on the menu: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "the printed sentence names no controller, so the Bird across the table \
         is on the menu too: {options:?}"
    );
    assert!(
        !options.contains(&elves),
        "the Elves are an Elf and none of the six subtypes: {options:?}"
    );
    assert!(
        !options.contains(&sanctuary),
        "the Sanctuary is a land and no creature: {options:?}"
    );

    // CR 601.2c names the target first and CR 601.2h pays afterwards, so both
    // prices are still unpaid while this question stands.
    assert!(
        !is_tapped(&engine, sanctuary),
        "the {{T}} is the last step of the activation, not the first"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "and the {{2}} is still floating for the same reason"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the Bird the question offered was chosen");

    assert!(
        is_tapped(&engine, sanctuary),
        "{{T}} is half the price and is paid with the rest"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "and the {{2}} came out of the three mana that was floating"
    );
    assert!(
        !stack_is_empty(&engine),
        "putting a counter on a creature is no mana ability, so the ability is \
         waiting on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, mine, CounterKind::P1P1),
        1,
        "\"put a +1/+1 counter on target Bird\" — a counter and not a pump \
         until end of turn"
    );
    assert_eq!(
        pt(&engine, mine),
        (before.0 + 1, before.1 + 1),
        "and the counter reaches the body it was put on"
    );
    assert_eq!(
        counters_on(&engine, theirs, CounterKind::P1P1),
        0,
        "the Bird nobody named got nothing: the ability targets, it does not \
         sweep the board"
    );
    assert_eq!(
        pt(&engine, theirs),
        before_theirs,
        "and is exactly the creature it was before the ability resolved"
    );
    assert_eq!(
        counters_on(&engine, sanctuary, CounterKind::P1P1),
        0,
        "the Sanctuary is the source and no creature: nothing counts it"
    );
}
