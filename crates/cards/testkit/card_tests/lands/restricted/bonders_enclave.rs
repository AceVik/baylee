//! `cards/lands/restricted/bonders_enclave.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Bonders' Enclave: "{T}: Add {C}." / "{3}, {T}: Draw a card. Activate only if you control a creature with power 4 or greater."
/// Under `Coverage::Partial`, the conditional card draw ability is omitted.
/// The land taps for its base mana ability to add {C} to the mana pool.
#[test]
fn bonders_enclave_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(130, forest())
        .battlefield(0, &[bonders_enclave()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let enclave = on_battlefield(&engine, p0, bonders_enclave()).expect("Enclave deployed");
    activate(&mut engine, p0, bonders_enclave(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, enclave));
}

/// Bonders' Enclave: "{3}, {T}: Draw a card. Activate only if you control a
/// creature with power 4 or greater." The gate is an activation condition
/// (CR 602.5b) and not a cost, so what it changes is the **offer**: with
/// three mana floating and only a 1/1 standing, the ability is not on the
/// list at all, and the same board with a 6/6 on it offers and pays it.
#[test]
fn bonders_enclave_draws_only_while_a_four_power_creature_stands() {
    let p0 = PlayerId::new(0);

    let mut small = Duel::new(9201, forest())
        .battlefield(
            0,
            &[
                bonders_enclave(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .start();
    keep_mulligans(&mut small);
    reach_main_phase(&mut small, p0);
    let enclave = on_battlefield(&small, p0, bonders_enclave()).expect("the Enclave is seated");
    tap_mana_except(&mut small, p0, enclave);
    assert!(
        small.state().players[0].mana_pool.total() >= 3,
        "the {{3}} is floating, so an absent ability is the condition and not the price"
    );
    let Pending::Priority { legal, .. } = small.pending().clone() else {
        panic!("expected priority, got {:?}", small.pending())
    };
    assert!(
        !legal.abilities.contains(&(enclave, 1)),
        "a 1/1 is not \"a creature with power 4 or greater\": {:?}",
        legal.abilities
    );
    assert!(
        legal.abilities.contains(&(enclave, 0)),
        "while the mana ability, which prints no condition, is offered"
    );

    let mut big = Duel::new(9202, forest())
        .battlefield(
            0,
            &[
                bonders_enclave(),
                forest(),
                forest(),
                forest(),
                rootbreaker_wurm(),
            ],
        )
        .start();
    keep_mulligans(&mut big);
    reach_main_phase(&mut big, p0);
    let enclave = on_battlefield(&big, p0, bonders_enclave()).expect("the Enclave is seated");
    tap_mana_except(&mut big, p0, enclave);
    let library_before = library_size(&big, p0);
    activate(&mut big, p0, bonders_enclave(), 1);
    pass_until(&mut big, stack_is_empty);
    assert_eq!(
        library_size(&big, p0),
        library_before - 1,
        "with a 6/6 standing the same three mana buys the card"
    );
    assert!(is_tapped(&big, enclave));
}
