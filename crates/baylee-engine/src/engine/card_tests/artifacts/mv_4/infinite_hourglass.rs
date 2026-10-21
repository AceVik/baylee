//! `cards/artifacts/mv_4/infinite_hourglass.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn infinite_hourglass() -> CardIndex {
    card_index("7c59ad65-ad1f-4dc4-bcf2-ea6b2b48fad8")
}

/// Infinite Hourglass (7c59ad65-ad1f-4dc4-bcf2-ea6b2b48fad8) — {4} Artifact:
/// - "At the beginning of your upkeep, put a time counter on this artifact."
/// - "{3}: Remove a time counter from this artifact. Any player may activate this ability but only during any upkeep step."
#[test]
fn infinite_hourglass_adds_time_counter_on_upkeep_and_removes_one_only_during_an_upkeep_step() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[infinite_hourglass(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);

    let hourglass = on_battlefield(&engine, P0, infinite_hourglass()).expect("Hourglass seated");

    // In P0's upkeep, the upkeep trigger resolves and puts 1 time counter on Infinite Hourglass.
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    assert_eq!(
        engine
            .state()
            .object(hourglass)
            .unwrap()
            .counters
            .get(CounterKind::Time),
        1,
        "upkeep trigger puts 1 time counter on Infinite Hourglass"
    );
    assert!(
        ability_offered(&engine, hourglass, 1),
        "{{3}} ability is offered during P0's upkeep step"
    );

    // Move to P0's main phase: the {3} ability is NOT offered outside an upkeep step.
    reach_main_phase(&mut engine, P0);
    assert!(
        !ability_offered(&engine, hourglass, 1),
        "{{3}} ability is not offered during a main phase"
    );

    // Advance to P1's upkeep step: "during any upkeep step" allows activation during P1's upkeep too.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1
            && e.state().turn.step == Step::Upkeep
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    assert!(
        ability_offered(&engine, hourglass, 1),
        "{{3}} ability is offered during P1's upkeep step"
    );
    tap_all_mana_but(&mut engine, P0, None);
    activate(&mut engine, P0, infinite_hourglass(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .object(hourglass)
            .unwrap()
            .counters
            .get(CounterKind::Time),
        0,
        "activating the {{3}} ability removes a time counter from Infinite Hourglass"
    );
}
