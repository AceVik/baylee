//! `cards/enchantments/mv_3/captive_flame.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Captive Flame costs `{2}{R}` and prints an activation whose entire
/// cost is one red mana: "{R}: Target creature gets +1/+0 until end of
/// turn." The scenario plays both halves in one main phase: four Mountains
/// pay the `{2}{R}` and leave exactly the red mana floating that the
/// ability then spends — `legal.abilities` is filtered behind
/// `can_afford`, so the line would not even be in the offer without that
/// mana. The Elf on the table is the control: "target creature" points at
/// every creature, but the `+1/+0` may only remain on the one that was
/// named; and the enchantment itself is not a creature and therefore does
/// not appear on the menu.
#[test]
fn captive_flame_pumps_only_the_creature_it_targets_for_one_red() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
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
        .hand(0, &[captive_flame()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(pt(&engine, mine), (1, 1), "a printed 1/1 before the pump");
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and so is the one across the table"
    );

    // The four Mountains into the pool and the Elf named as the one that
    // remains: its own `{T}: Add {G}` is also a mana ability, whose entire
    // cost is its own tap, and a green mana in the pool would extend every
    // number below by a source that has nothing to do with it.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        4,
        "four Mountains tapped, and no Elf contributed a green"
    );
    cast_with_floating(&mut engine, p0, captive_flame());
    pass_until(&mut engine, stack_is_empty);

    let flame = on_battlefield(&engine, p0, captive_flame()).expect("the Flame resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{2}}{{R}} is spent and exactly the {{R}} the ability charges is left"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(flame, 0)),
        "with a red already floating the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, captive_flame(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that activated chooses");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&flame),
        "an enchantment is no creature, so the Flame cannot pump itself: {options:?}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "CR 601.2c names the target first: nothing is spent while the question stands"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the creature the question offered was named");

    // CR 601.2h: the `{R}` is the last step of the activation, so the pool
    // is only read as empty here — and the enchantment stays untapped,
    // because its cost is the mana and never its own `{T}`.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{R}} came out of the pool"
    );
    assert!(!is_tapped(&engine, flame), "no {{T}} is part of the cost");
    assert!(!stack_is_empty(&engine), "the pump is no mana ability");

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, mine), (2, 1), "+1/+0 on the creature it named");
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and nothing at all for a creature it did not"
    );
}
