//! `cards/instants/mv_1/shrink.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Shrink is `{G}` for a single printed sentence: "Target creature gets
/// -5/-0 until end of turn." On the battlefield stands a printed 6/6 Wurm, so
/// that the −5 lands on a number that remains positive: `(1, 6)` reads both
/// halves of the calculation — a `(1, 1)` would mean that a toughness
/// penalty was paid along with it, which the card does not print. The Elves
/// next to it and the Elves across the table are the two controls: one
/// shows that the static only affects the *named* creature, the other that
/// "target creature" was not narrowed to "your creature". The second main
/// phase visit reads the duration, which is half the card: "until end of
/// turn" is over as soon as the turn in which it was cast is over.
#[test]
fn shrink_takes_five_power_from_the_creature_it_names_until_the_turn_ends() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), rootbreaker_wurm(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[shrink()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let wurm = on_battlefield(&engine, p0, rootbreaker_wurm()).expect("the Wurm is out");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(pt(&engine, wurm), (6, 6), "a printed 6/6 before the spell");

    // The Forest pays the {G}; the Elves are named as the printing that
    // remains, because they are the creature that the control below reads.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one Forest tapped, one green floating"
    );
    cast_with_floating(&mut engine, p0, shrink());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until only stops on a target choice")
    };
    assert_eq!(player, p0, "the caster aims it");
    assert!(
        options.contains(&wurm) && options.contains(&elves) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wurm],
            },
        )
        .expect("the Wurm was one of the options it enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, wurm),
        (1, 6),
        "−5/−0 on the creature it named — a (1, 1) would be a toughness \
         penalty the card never prints"
    );
    assert_eq!(
        pt(&engine, elves),
        (1, 1),
        "and nothing at all for the creature it did not name"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "nor for the creature across the table, which it merely could have named"
    );

    // The other half of the sentence. The next main phase lies behind
    // the end of p0's turn, and exactly there "until end of turn" expires.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        pt(&engine, wurm),
        (6, 6),
        "the pump lasts until end of turn, and this is no longer that turn"
    );
}
