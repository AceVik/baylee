//! `cards/lands/utility/oran_rief_the_vastwood.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Oran-Rief, the Vastwood: "This land enters tapped." / "{T}: Add {G}." / "{T}: Put a +1/+1 counter on each green creature that entered this turn."
/// Under `Coverage::Partial`, the turn-history creature pump is omitted.
/// Playing the land from hand verifies that it enters tapped, and on the following turn it untaps and taps for {G}.
#[test]
fn oran_rief_the_vastwood_enters_tapped_and_taps_for_green() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(105, forest())
        .hand(0, &[oran_rief_the_vastwood()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, oran_rief_the_vastwood());
    assert!(entered_tapped(&engine, land), "Oran-Rief enters tapped");

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land), "untaps on next turn");

    activate(&mut engine, p0, oran_rief_the_vastwood(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1
    );
    assert!(is_tapped(&engine, land));
}

/// Oran-Rief, the Vastwood prints `This land enters tapped.`, `{{T}}: Add {{G}}.`, and
/// `{{T}}: Put a +1/+1 counter on each green creature that entered this turn.`
/// Under `Coverage::Implemented`, both abilities and the entry modifier are supported.
/// This test plays Oran-Rief tapped, untaps it on the next turn, floats mana to cast both a green
/// creature (`young_wolf()`) and a colorless creature (`myr_retriever()`), activates ability 1 to place
/// a +1/+1 counter on the newly arrived green creature while leaving the colorless creature and a seeded
/// green creature (`llanowar_elves()`) untouched, and taps for green mana via ability 0 on the next turn.
#[test]
fn oran_rief_the_vastwood_enters_tapped_and_puts_counter_on_arrived_green_creatures() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), llanowar_elves()])
        .hand(
            0,
            &[oran_rief_the_vastwood(), young_wolf(), myr_retriever()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, oran_rief_the_vastwood());
    assert!(
        entered_tapped(&engine, land),
        "Oran-Rief, the Vastwood enters tapped"
    );

    // Untap on the next turn.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land), "land untaps on next turn");

    let seeded_elf =
        on_battlefield(&engine, p0, llanowar_elves()).expect("seeded elf on battlefield");

    // Float mana keeping Oran-Rief untapped: 3 Forests and 1 Llanowar Elves provide 4 green mana.
    tap_all_mana_but(&mut engine, p0, Some(oran_rief_the_vastwood()));
    cast_with_floating(&mut engine, p0, young_wolf());
    pass_until(&mut engine, stack_is_empty);
    cast_with_floating(&mut engine, p0, myr_retriever());
    pass_until(&mut engine, stack_is_empty);

    let wolf = on_battlefield(&engine, p0, young_wolf()).expect("wolf on battlefield");
    let myr = on_battlefield(&engine, p0, myr_retriever()).expect("myr on battlefield");

    // Activate ability 1 ({T} for counter on each green creature that entered this turn).
    activate(&mut engine, p0, oran_rief_the_vastwood(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, wolf, CounterKind::P1P1),
        1,
        "green creature that entered this turn gets a +1/+1 counter"
    );
    assert_eq!(pt(&engine, wolf), (2, 2), "grows to a 2/2");
    assert_eq!(
        counters_on(&engine, myr, CounterKind::P1P1),
        0,
        "colorless creature that entered this turn is not green"
    );
    assert_eq!(pt(&engine, myr), (1, 1));
    assert_eq!(
        counters_on(&engine, seeded_elf, CounterKind::P1P1),
        0,
        "seeded green creature did not enter this turn"
    );
    assert_eq!(pt(&engine, seeded_elf), (1, 1));
    assert!(is_tapped(&engine, land), "Oran-Rief is tapped");

    // On the following turn, ability 0 taps for green mana.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, oran_rief_the_vastwood(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "ability 0 adds {{G}}"
    );
    assert!(is_tapped(&engine, land));
}
