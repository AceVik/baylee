//! `cards/artifacts/mv_4/infinite_hourglass.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn infinite_hourglass() -> CardIndex {
    card_index("7c59ad65-ad1f-4dc4-bcf2-ea6b2b48fad8")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Infinite Hourglass (7c59ad65-ad1f-4dc4-bcf2-ea6b2b48fad8) — {4} Artifact:
/// "At the beginning of your upkeep, put a time counter on this artifact."
/// "All creatures get +1/+0 for each time counter on this artifact."
/// "{3}: Remove a time counter from this artifact. Any player may activate
///  this ability but only during any upkeep step."
///
/// Verifies upkeep counter placement (yours only), global +1/+0 scaling on
/// both sides, upkeep-only timing restriction (refused in main phase), and
/// activation by both the opponent and the controller during upkeep.
#[test]
fn infinite_hourglass_scales_all_creatures_and_any_player_removes_during_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                infinite_hourglass(),
                grizzly_bears(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .battlefield(1, &[llanowar_elves(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);

    let hourglass = on_battlefield(&engine, p0, infinite_hourglass()).unwrap();
    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let elves = on_battlefield(&engine, p1, llanowar_elves()).unwrap();

    // 1. p0's upkeep trigger puts a time counter on Infinite Hourglass -> all creatures +1/+0.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0
            && e.state().turn.step == Step::Upkeep
            && counters_on(e, hourglass, CounterKind::Time) == 1
            && stack_is_empty(e)
    });
    assert_eq!(pt(&engine, bears), (3, 2), "own creature gets +1/+0");
    assert_eq!(
        pt(&engine, elves),
        (2, 1),
        "opponent's creature also gets +1/+0"
    );

    // 2. Main phase: even with {3} floating, removing a time counter is NOT offered.
    reach_main_phase(&mut engine, p0);
    tap_all_mana(&mut engine, p0);
    assert!(
        !ability_offered(&engine, hourglass, 2),
        "removal ability is upkeep-only, not offered in main phase"
    );

    // 3. Opponent's upkeep: no time counter added (yours only), and opponent CAN pay {3} to remove one.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1
            && e.state().turn.step == Step::Upkeep
            && e.pending().asked() == Some(p1)
    });
    assert_eq!(
        counters_on(&engine, hourglass, CounterKind::Time),
        1,
        "opponent's upkeep does not add a time counter"
    );
    tap_all_mana(&mut engine, p1);
    assert!(
        ability_offered(&engine, hourglass, 2),
        "any player may activate during any upkeep"
    );
    engine
        .apply(
            p1,
            PlayerAction::ActivateAbility {
                source: hourglass,
                ability_index: 2,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, hourglass, CounterKind::Time), 0);
    assert_eq!(pt(&engine, bears), (2, 2));
    assert_eq!(pt(&engine, elves), (1, 1));

    // 4. p0's next upkeep adds a counter again, and p0 can also pay {3} during upkeep to remove it.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0
            && e.state().turn.step == Step::Upkeep
            && counters_on(e, hourglass, CounterKind::Time) == 1
            && stack_is_empty(e)
            && e.pending().asked() == Some(p0)
    });
    assert_eq!(pt(&engine, bears), (3, 2));
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, infinite_hourglass(), 2);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, hourglass, CounterKind::Time), 0);
    assert_eq!(pt(&engine, bears), (2, 2));
}
