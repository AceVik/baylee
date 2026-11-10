//! `cards/creatures/artifacts/mv_4/soldevi_simulacrum.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn soldevi_simulacrum() -> CardIndex {
    card_index("227ed0b7-7a9a-41f1-a190-1d1b85a44720")
}

/// Soldevi Simulacrum (227ed0b7-7a9a-41f1-a190-1d1b85a44720) — {4} 2/4 Artifact Creature — Soldier:
/// - "Cumulative upkeep {1}"
/// - "{1}: This creature gets +1/+0 until end of turn."
#[test]
fn soldevi_simulacrum_pumps_power_and_scales_cumulative_upkeep_before_sacrificing() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[soldevi_simulacrum(), plains(), plains()])
        .start();
    keep_mulligans(&mut engine);

    let sim = on_battlefield(&engine, P0, soldevi_simulacrum()).unwrap();
    assert_eq!(pt(&engine, sim), (2, 4));

    // Upkeep 1 (turn 1): 1 age counter -> pay {1} with one Plains.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    let pl = on_battlefield(&engine, P0, plains()).unwrap();
    tap_mana_where(&mut engine, P0, |id| id == pl);
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    assert_eq!(
        engine
            .state()
            .object(sim)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::AGE),
        1
    );

    // Main phase: spend the remaining Plains on {1}: +1/+0 until end of turn -> 3/4.
    reach_main_phase(&mut engine, P0);
    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, soldevi_simulacrum(), 1);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, sim), (3, 4));

    // Upkeep 2 (turn 3): 2nd age counter -> pay {2} with both Plains, and verify previous turn's pump wore off.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 3 && matches!(e.pending(), Pending::YesNo { .. })
    });
    assert_eq!(pt(&engine, sim), (2, 4));
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    tap_all_mana(&mut engine, P0);
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.number == 3 && e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    assert_eq!(
        engine
            .state()
            .object(sim)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::AGE),
        2
    );

    // Upkeep 3 (turn 5): 3rd age counter -> decline payment -> sacrificed.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 5 && matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.number == 5 && e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    assert!(
        on_battlefield(&engine, P0, soldevi_simulacrum()).is_none()
            && in_graveyard(&engine, P0, soldevi_simulacrum()).is_some(),
        "Soldevi Simulacrum is sacrificed when cumulative upkeep is declined"
    );
}
