//! `cards/artifacts/mv_4/lux_cannon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn lux_cannon() -> CardIndex {
    card_index("ebdf26a4-77ee-4635-8d52-926bdca623f7")
}

/// "{T}: Put a charge counter on this artifact."
#[test]
fn tapping_the_cannon_charges_it() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[lux_cannon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let cannon = on_battlefield(&engine, p0, lux_cannon()).unwrap();
    activate(&mut engine, p0, lux_cannon(), 0);
    assert!(is_tapped(&engine, cannon));
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, cannon, CounterKind::Charge), 1);
}

/// "{T}, Remove three charge counters from this artifact: Destroy target
/// permanent." Two counters do not pay; three do, and any permanent of
/// either side is a target.
#[test]
fn three_counters_destroy_a_target_permanent() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[lux_cannon()])
        .battlefield(1, &[quiet_artifact(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let cannon = on_battlefield(&engine, p0, lux_cannon()).unwrap();
    let rock = on_battlefield(&engine, p1, quiet_artifact()).unwrap();
    let land = on_battlefield(&engine, p1, forest()).unwrap();

    set_counters(&mut engine, cannon, CounterKind::Charge, 2);
    assert!(
        !priority_offer(&engine).abilities.contains(&(cannon, 1)),
        "two counters cannot pay for three"
    );
    set_counters(&mut engine, cannon, CounterKind::Charge, 3);
    activate(&mut engine, p0, lux_cannon(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("a target is asked for, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&rock) && options.contains(&land),
        "{options:?}"
    );
    assert!(options.contains(&cannon), "any permanent, even its own");
    aim_at(&mut engine, p0, rock);
    assert!(is_tapped(&engine, cannon));
    assert_eq!(counters_on(&engine, cannon, CounterKind::Charge), 0);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p1, quiet_artifact()).is_some());
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "only the target"
    );
}
