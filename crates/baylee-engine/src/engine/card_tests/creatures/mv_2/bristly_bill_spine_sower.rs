//! `cards/creatures/mv_2/bristly_bill_spine_sower.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Bristly Bill, Spine Sower` prints `Landfall — Whenever a land you control enters, put a +1/+1 counter on target creature.` and `{{3}}{{G}}{{G}}: Double the number of +1/+1 counters on each creature you control.`
///
/// Entering lands trigger `Trigger::EntersBattlefield` with `Filter::YOUR_LAND`, placing a
/// `CounterKind::P1P1` on target creature via `Pending::ChooseTargets`. The doubling then gives
/// each creature its controller controls as many counters as it has (CR 701.10e): Bill's one
/// becomes two, the Elf's none stays none, and the opponent's creature keeps its two, because it
/// is not a creature Bill's controller controls. This test pinned the missing activation until
/// `Effect::DoubleCountersFilter` said it.
#[test]
fn bristly_bill_spine_sower_triggers_landfall_counter_and_doubles_its_own() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                bristly_bill_spine_sower(),
                llanowar_elves(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .battlefield(1, &[aurochs()])
        .hand(0, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bill =
        on_battlefield(&engine, p0, bristly_bill_spine_sower()).expect("bill on battlefield");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is seated");
    let theirs = on_battlefield(&engine, p1, aurochs()).expect("the opponent's Aurochs");
    crate::replacement::put_counters(
        engine
            .dev_state_mut(p0)
            .expect("the harness sets boards up"),
        theirs,
        CounterKind::P1P1,
        2,
    );
    assert_eq!(pt(&engine, bill), (2, 2));
    assert_eq!(counters_on(&engine, bill, CounterKind::P1P1), 0);

    play_land(&mut engine, p0, forest());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets prompt, got {:?}", engine.pending());
    };
    assert!(
        options.contains(&bill),
        "Bristly Bill is a legal target creature"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bill],
                players: vec![],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, bill, CounterKind::P1P1), 1);
    assert_eq!(pt(&engine, bill), (3, 3));

    let forests = all_of(&engine, p0, forest());
    tap_mana_where(&mut engine, p0, |id| forests.contains(&id));
    activate(&mut engine, p0, bristly_bill_spine_sower(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, bill, CounterKind::P1P1),
        2,
        "one counter doubled is two"
    );
    assert_eq!(pt(&engine, bill), (4, 4));
    assert_eq!(
        counters_on(&engine, elf, CounterKind::P1P1),
        0,
        "none doubled is none"
    );
    assert_eq!(
        counters_on(&engine, theirs, CounterKind::P1P1),
        2,
        "the opponent's creature is not one Bill's controller controls"
    );
}
