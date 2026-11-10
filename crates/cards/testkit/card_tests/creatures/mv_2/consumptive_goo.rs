//! `cards/creatures/mv_2/consumptive_goo.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Consumptive Goo` prints an activated ability `{{2}}{{B}}{{B}}` under `Coverage::Implemented`:
/// "Target creature gets -1/-1 until end of turn. Put a +1/+1 counter on this creature."
/// Activating this ability targeting an opponent's creature verifies that the target receives
/// -1/-1 until end of turn while `Consumptive Goo` gains a `CounterKind::P1P1` counter.
#[test]
fn consumptive_goo_shrinks_target_and_gains_counter() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(1608, forest())
        .battlefield(0, &[consumptive_goo(), swamp(), swamp(), swamp(), swamp()])
        .battlefield(1, &[giant_tortoise()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let goo = on_battlefield(&engine, p0, consumptive_goo()).expect("goo is on battlefield");
    let tortoise =
        on_battlefield(&engine, p1, giant_tortoise()).expect("tortoise is on battlefield");

    assert_eq!(pt(&engine, goo), (1, 1));
    assert_eq!(pt(&engine, tortoise), (1, 4));

    tap_all_mana(&mut engine, p0);

    activate(&mut engine, p0, consumptive_goo(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice for Consumptive Goo");
    };
    assert!(options.contains(&tortoise));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![tortoise],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(counters_on(&engine, goo, CounterKind::P1P1), 1);
    assert_eq!(pt(&engine, goo), (2, 2));
    assert_eq!(pt(&engine, tortoise), (0, 3));
}
