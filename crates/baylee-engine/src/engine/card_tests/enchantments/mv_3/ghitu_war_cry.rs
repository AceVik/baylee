//! `cards/enchantments/mv_3/ghitu_war_cry.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ghitu War Cry — {2}{R} Enchantment: "{R}: Target creature gets +1/+0
/// until end of turn." The whole card is that line, so the scenario has to
/// read it in three places at once: the {2}{R} out of a pool four Mountains
/// filled, the {R} for the activation that leaves the pool at zero, and the
/// pump itself, which must land on the targeted Elf and on no other. Two
/// Elves stand — one under each seat — because "target creature" is not
/// "creatures you control", and a static that had reached the whole table
/// would still satisfy a scenario holding only the caster's own board.
#[test]
fn ghitu_war_cry_taps_a_mountain_to_pump_the_creature_it_targets() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(8311, forest())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[ghitu_war_cry()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert_eq!(pt(&engine, mine), (1, 1), "a printed 1/1 before the pump");
    assert_eq!(pt(&engine, theirs), (1, 1), "and one across the table");

    // The Elf is named as the source kept back: it is the creature the
    // ability is about to target, and a host tapped for its own mana would
    // no longer be the permanent this test reads afterwards.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Mountains, and no Elf of mine paid in"
    );

    // {2}{R} off the pool, which leaves exactly the {R} the ability charges
    // inside this one main phase (CR 500.5).
    cast_with_floating(&mut engine, p0, ghitu_war_cry());
    pass_until(&mut engine, stack_is_empty);
    let cry = on_battlefield(&engine, p0, ghitu_war_cry()).expect("the enchantment resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{2}}{{R}} is spent and one red is left floating for the ability"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(cry, 0)),
        "with the {{R}} in the pool the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, ghitu_war_cry(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that chooses");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&cry),
        "the enchantment is no creature and cannot pump itself: {options:?}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "CR 601.2h pays last: nothing is spent while the question stands"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the creature the question offered was chosen");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{R}} came out of the pool"
    );
    assert!(!stack_is_empty(&engine), "pumping is no mana ability");

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, mine),
        (2, 1),
        "+1/+0 until end of turn on the creature it targeted"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the ability reaches the creature it named and never across the table"
    );
    assert!(
        on_battlefield(&engine, p0, ghitu_war_cry()).is_some(),
        "an activation costs the enchantment nothing but mana"
    );
}
