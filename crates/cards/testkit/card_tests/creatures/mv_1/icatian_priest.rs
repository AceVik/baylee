//! `cards/creatures/mv_1/icatian_priest.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Icatian Priest — {W}, a 1/1 Human Cleric whose entire text is
/// "{1}{W}{W}: Target creature gets +1/+1 until end of turn."
///
/// Three creatures stand on the two battlefields on purpose, because the
/// printed words are "target creature" and nothing narrower: the pump has to
/// land on the one the ability names and leave the two it did not name at
/// their printed 1/1 — one beside it and one across the table, so "any
/// creature" is read rather than assumed. The price is read off the *pool*,
/// before and after the target is answered, because CR 601.2c picks the
/// target and CR 601.2h pays last: four mana float when the menu opens and one
/// when the cost lands, and that leftover one is what then refuses the same
/// ability, which is what says the price is three mana and not a tap.
#[test]
fn icatian_priest_pumps_the_creature_it_names_and_only_that_one() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                quiet_creature(),
            ],
        )
        .battlefield(1, &[quiet_creature()])
        .hand(0, &[icatian_priest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The Elf beside the Priest is a mana source of its own, so it is named as
    // the one thing kept back: `tap_all_mana` would take it too and the pool
    // below would be six mana rather than the five Plains that pay for both
    // the cast and the activation.
    tap_all_mana_but(&mut engine, p0, Some(quiet_creature()));
    cast_with_floating(&mut engine, p0, icatian_priest());
    pass_until(&mut engine, stack_is_empty);

    let priest = on_battlefield(&engine, p0, icatian_priest()).expect("the Priest resolved");
    let mine = on_battlefield(&engine, p0, quiet_creature()).expect("my Elf is out");
    let theirs = on_battlefield(&engine, p1, quiet_creature()).expect("their Elf is out");
    assert_eq!(pt(&engine, priest), (1, 1), "the 1/1 body the card prints");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "five Plains paid the {{W}} and left four, which is more than the \
         {{1}}{{W}}{{W}} the ability asks"
    );

    // The offer is a pool reading (`can_afford`), so the mana is floating
    // before the ability is claimed to be offered.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(priest, 0)),
        "an untapped Priest and {{1}}{{W}}{{W}} in the pool: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, icatian_priest(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("the pump targets a creature, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat names the creature");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" reaches either side of the table: {options:?}"
    );
    assert!(
        options.contains(&priest),
        "the Priest is a creature itself, so it is on its own menu: {options:?}"
    );
    assert_eq!(options.len(), 3, "three creatures, three legal targets");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the creature the question offered is the target");

    // CR 601.2h: the cost is the last step of the activation, so the pool is
    // read *after* the target answer and not before it.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "{{1}}{{W}}{{W}} came out of the four that were floating"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, mine),
        (2, 2),
        "+1/+1 on the creature the ability named"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and nothing across the table: the pump is aimed"
    );
    assert_eq!(
        pt(&engine, priest),
        (1, 1),
        "nor on the Priest that was never named"
    );

    // One mana is left and three is the price, so the very ability that was
    // offered a moment ago is not offered now — the pool saying so, and not
    // the board.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(priest, 0)),
        "one mana is not {{1}}{{W}}{{W}}: {:?}",
        legal.abilities
    );
}
