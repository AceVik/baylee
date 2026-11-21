//! `cards/artifacts/mv_2/energy_chamber.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn energy_chamber() -> CardIndex {
    card_index("c0880a44-27d5-4e21-915c-678ee46fa3d9")
}

fn steel_wall() -> CardIndex {
    card_index("5ccb57e1-ca94-4b5a-8e5f-b8b5e692cfb9")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Energy Chamber (c0880a44-27d5-4e21-915c-678ee46fa3d9) — {2} Artifact:
/// "At the beginning of your upkeep, choose one —
///  • Put a +1/+1 counter on target artifact creature.
///  • Put a charge counter on target noncreature artifact."
///
/// Mode 0 targets only artifact creatures, opponent's upkeep does not trigger
/// Energy Chamber, and Mode 1 targets only noncreature artifacts.
#[test]
fn energy_chamber_both_modes_and_upkeep_ownership() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[energy_chamber(), steel_wall(), sol_ring(), grizzly_bears()],
        )
        .start();
    keep_mulligans(&mut engine);

    let chamber = on_battlefield(&engine, p0, energy_chamber()).unwrap();
    let wall = on_battlefield(&engine, p0, steel_wall()).unwrap();
    let ring = on_battlefield(&engine, p0, sol_ring()).unwrap();
    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();

    // Advance to p0's upkeep: modal trigger asks ChooseCastMode.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    assert_eq!(engine.state().turn.active, p0);
    assert_eq!(engine.state().turn.step, Step::Upkeep);

    // Mode 0: +1/+1 counter on target artifact creature (Steel Wall only).
    engine.apply(p0, PlayerAction::ChooseMode(0)).unwrap();
    let menu0 = aim_at(&mut engine, p0, wall);
    assert!(menu0.contains(&wall), "artifact creature is legal");
    assert!(
        !menu0.contains(&bears) && !menu0.contains(&ring) && !menu0.contains(&chamber),
        "non-artifact creature and noncreature artifacts are excluded: {menu0:?}"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, wall, CounterKind::P1P1), 1);
    assert_eq!(pt(&engine, wall), (1, 5));

    // Opponent's upkeep must NOT trigger Energy Chamber.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.step == Step::Upkeep
    });
    assert!(
        !matches!(engine.pending(), Pending::ChooseCastMode { .. }) && stack_is_empty(&engine),
        "Energy Chamber does not trigger on the opponent's upkeep"
    );

    // p0's next upkeep: Mode 1 puts a charge counter on target noncreature artifact.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    engine.apply(p0, PlayerAction::ChooseMode(1)).unwrap();
    let menu1 = aim_at(&mut engine, p0, ring);
    assert!(
        menu1.contains(&ring) && menu1.contains(&chamber),
        "noncreature artifacts are legal: {menu1:?}"
    );
    assert!(
        !menu1.contains(&wall) && !menu1.contains(&bears),
        "artifact creature and non-artifact creature are excluded: {menu1:?}"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, ring, CounterKind::Charge), 1);
    assert_eq!(counters_on(&engine, wall, CounterKind::Charge), 0);
}
