//! `cards/creatures/mv_3/marker_beetles.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Marker Beetles prints two sentences and one play makes both happen at
/// once. `{2}, Sacrifice this creature: Draw a card` is the only way to kill
/// it on demand, and a creature sacrificed to a cost still *dies* (CR 700.4)
/// — so the sacrifice turns on the second line, "When this creature dies,
/// target creature gets +1/+1 until end of turn", which lands on the stack
/// above the draw. Answering its target question with the Elf under the same
/// seat and reading the Elf across the table back at (1, 1) is what tells
/// "target creature" from "creatures you control"; the library one shorter
/// and the hand one longer are the third clause, and the pool emptied to zero
/// says the `{2}` was a real price and not a label.
#[test]
fn marker_beetles_sacrifices_itself_to_draw_and_pumps_a_creature_when_it_dies() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), llanowar_elves()],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[marker_beetles()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Five sources pay {{1}}{{G}}{{G}} and leave exactly the {{2}} the second
    // line charges. The Elf is one of them, which changes nothing about the
    // body the dies trigger is aimed at.
    cast_from_hand(&mut engine, p0, marker_beetles());
    pass_until(&mut engine, stack_is_empty);
    let beetles = on_battlefield(&engine, p0, marker_beetles()).expect("the Beetles resolved");
    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert_eq!(pt(&engine, mine), (1, 1), "a printed 1/1 before the pump");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the cast is paid and the {{2}} the ability charges is still floating"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(beetles, 0)),
        "with the {{2}} in the pool the sacrifice line is offered: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // Ability 0 is the activated line and ability 1 is the trigger behind it.
    activate(&mut engine, p0, marker_beetles(), 0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}} came out of the pool"
    );
    assert!(
        in_graveyard(&engine, p0, marker_beetles()).is_some(),
        "CR 601.2h: the sacrifice is a cost, so the Beetles are gone before \
         anything resolves"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until only stops on a target choice")
    };
    assert_eq!(
        player, p0,
        "the seat that sacrificed the Beetles answers its own trigger"
    );
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&beetles),
        "the Beetles are in a graveyard and no creature to target at all: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the creature the question offered was chosen");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, mine),
        (2, 2),
        "\"target creature gets +1/+1 until end of turn\""
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the pump reaches the creature that was named and never across the table"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "and it reached the hand, so a library that emptied would not satisfy \
         the count above"
    );
}
