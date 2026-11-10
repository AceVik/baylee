//! `cards/lands/utility/novijen_heart_of_progress.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Novijen, Heart of Progress: "{T}: Add {C}." / "{G}{U}, {T}: Put a +1/+1 counter on each creature that entered this turn."
/// Under `Coverage::Partial`, filtering creatures by entry time is unsupported and omitted.
/// The land taps to add {C} to the mana pool and offers no second ability.
#[test]
fn novijen_heart_of_progress_taps_for_colorless_and_omits_counter_ability() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(128, forest())
        .battlefield(0, &[novijen_heart_of_progress()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let novijen =
        on_battlefield(&engine, p0, novijen_heart_of_progress()).expect("Novijen deployed");

    activate(&mut engine, p0, novijen_heart_of_progress(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, novijen));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal
            .abilities
            .iter()
            .any(|(s, ai)| *s == novijen && *ai == 1),
        "no second ability is offered"
    );
}

/// Novijen, Heart of Progress prints `{{T}}: Add {{C}}.` and
/// `{{G}}{{U}}, {{T}}: Put a +1/+1 counter on each creature that entered this turn.`
/// Under `Coverage::Implemented`, both abilities are supported.
/// This test verifies that activating ability 1 with `{{G}}{{U}}` floating puts a +1/+1
/// counter on a creature that entered this turn (`young_wolf()`), while excluding a seeded
/// creature (`llanowar_elves()`) and an opponent's seeded creature from earlier turns.
/// On the subsequent turn, Novijen untaps and taps for colorless mana via ability 0.
#[test]
fn novijen_heart_of_progress_adds_counters_to_each_creature_entered_this_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                novijen_heart_of_progress(),
                forest(),
                forest(),
                island(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[young_wolf()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let novijen =
        on_battlefield(&engine, p0, novijen_heart_of_progress()).expect("Novijen on battlefield");
    let seeded_elf =
        on_battlefield(&engine, p0, llanowar_elves()).expect("seeded elf on battlefield");
    let opp_elf =
        on_battlefield(&engine, p1, llanowar_elves()).expect("opponent elf on battlefield");

    assert!(!is_tapped(&engine, novijen), "Novijen enters untapped");
    assert_eq!(pt(&engine, seeded_elf), (1, 1));
    assert_eq!(pt(&engine, opp_elf), (1, 1));

    // Float mana keeping Novijen untapped: 2 Forests, 1 Island, and 1 Llanowar Elves
    // produce 3 green and 1 blue mana.
    tap_all_mana_but(&mut engine, p0, Some(novijen_heart_of_progress()));
    cast_with_floating(&mut engine, p0, young_wolf());
    pass_until(&mut engine, stack_is_empty);

    let wolf = on_battlefield(&engine, p0, young_wolf()).expect("young wolf on battlefield");
    assert_eq!(pt(&engine, wolf), (1, 1), "newly arrived Young Wolf is 1/1");

    // Activate ability 1 ({G}{U}, {T}) off the remaining floating mana.
    activate(&mut engine, p0, novijen_heart_of_progress(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, wolf, CounterKind::P1P1),
        1,
        "creature that entered this turn gets a +1/+1 counter"
    );
    assert_eq!(pt(&engine, wolf), (2, 2), "grows to a 2/2");
    assert_eq!(
        counters_on(&engine, seeded_elf, CounterKind::P1P1),
        0,
        "creature from setup did not enter this turn"
    );
    assert_eq!(pt(&engine, seeded_elf), (1, 1));
    assert_eq!(
        counters_on(&engine, opp_elf, CounterKind::P1P1),
        0,
        "opponent creature from setup did not enter this turn"
    );
    assert_eq!(pt(&engine, opp_elf), (1, 1));
    assert!(is_tapped(&engine, novijen), "Novijen is tapped");

    // On the following turn, Novijen untaps and taps for colorless mana via ability 0.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, novijen), "Novijen untaps on next turn");

    activate(&mut engine, p0, novijen_heart_of_progress(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1,
        "ability 0 adds {{C}}"
    );
    assert!(is_tapped(&engine, novijen));
}
