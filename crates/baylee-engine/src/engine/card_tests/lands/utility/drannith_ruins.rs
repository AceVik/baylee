//! `cards/lands/utility/drannith_ruins.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Drannith Ruins prints `{{T}}: Add {{C}}.` and
/// `{{2}}, {{T}}: Put two +1/+1 counters on target non-Human creature that entered this turn.`
/// Under `Coverage::Implemented`, both abilities are supported.
/// This test verifies that Drannith Ruins enters untapped, and when `{{2}}` floats alongside
/// casting both a Human creature (`drannith_magistrate()`) and a non-Human creature (`llanowar_elves()`),
/// ability 1 targets only the non-Human creature that entered this turn (excluding the Human and a
/// seeded non-Human creature (`young_wolf()`) from setup), placing two +1/+1 counters. On the following
/// turn, ability 0 taps for colorless mana.
#[test]
fn drannith_ruins_puts_two_counters_on_arrived_non_human_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                drannith_ruins(),
                plains(),
                plains(),
                forest(),
                forest(),
                forest(),
                young_wolf(),
            ],
        )
        .hand(0, &[drannith_magistrate(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ruins =
        on_battlefield(&engine, p0, drannith_ruins()).expect("Drannith Ruins on battlefield");
    let seeded_wolf =
        on_battlefield(&engine, p0, young_wolf()).expect("seeded wolf on battlefield");
    assert!(!is_tapped(&engine, ruins), "Drannith Ruins enters untapped");

    // Float mana keeping Drannith Ruins untapped (2 white and 3 green mana).
    tap_all_mana_but(&mut engine, p0, Some(drannith_ruins()));
    cast_with_floating(&mut engine, p0, drannith_magistrate());
    pass_until(&mut engine, stack_is_empty);
    cast_with_floating(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);

    let magistrate =
        on_battlefield(&engine, p0, drannith_magistrate()).expect("magistrate on battlefield");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("elf on battlefield");

    // Activate ability 1 ({2}, {T}).
    activate(&mut engine, p0, drannith_ruins(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert_eq!(
        options,
        vec![elf],
        "only the non-Human creature that entered this turn is a legal target"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("targeting the entered non-Human creature is legal");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, elf, CounterKind::P1P1),
        2,
        "receives two +1/+1 counters"
    );
    assert_eq!(pt(&engine, elf), (3, 3), "grows to a 3/3");
    assert_eq!(
        counters_on(&engine, magistrate, CounterKind::P1P1),
        0,
        "Human creature is not targeted"
    );
    assert_eq!(
        counters_on(&engine, seeded_wolf, CounterKind::P1P1),
        0,
        "seeded creature did not enter this turn"
    );
    assert!(is_tapped(&engine, ruins), "Drannith Ruins is tapped");

    // On the following turn, ability 0 taps for colorless mana.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, ruins));

    activate(&mut engine, p0, drannith_ruins(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1,
        "ability 0 adds {{C}}"
    );
    assert!(is_tapped(&engine, ruins));
}
