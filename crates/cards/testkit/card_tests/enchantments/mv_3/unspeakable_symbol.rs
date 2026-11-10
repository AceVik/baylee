//! `cards/enchantments/mv_3/unspeakable_symbol.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Unspeakable Symbol` is an enchantment costing `{1}{B}{B}` under `Coverage::Implemented`.
/// It prints "Pay 3 life: Put a +1/+1 counter on target creature."
/// Under CR 601.2c and CR 601.2h, targeting occurs before the life cost is deducted.
/// Upon completing activation, 3 life is paid, and upon resolution, the target creature
/// (such as `Llanowar Elves`) receives a +1/+1 counter.
#[test]
fn unspeakable_symbol_pays_life_to_place_counter_on_target_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[unspeakable_symbol(), llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elf = on_battlefield(&engine, p0, llanowar_elves())
        .expect("Llanowar Elves is on the battlefield");
    assert_eq!(counters_on(&engine, elf, CounterKind::P1P1), 0);

    activate(&mut engine, p0, unspeakable_symbol(), 0);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseTargets prompt for Unspeakable Symbol, got {:?}",
            engine.pending()
        );
    };
    assert!(
        options.contains(&elf),
        "Llanowar Elves is an offered target: {options:?}"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "life is not yet paid while target choice is pending"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();

    assert_eq!(
        engine.state().players[0].life,
        17,
        "paid 3 life upon finishing the activation"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, elf, CounterKind::P1P1),
        1,
        "placed a +1/+1 counter on the target creature"
    );
    assert_eq!(pt(&engine, elf), (2, 2), "creature body grew to 2/2");
}
