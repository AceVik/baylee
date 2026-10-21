//! `cards/enchantments/mv_3/snowfall.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn snowfall() -> CardIndex {
    card_index("5ffed544-1656-4753-ac8f-ad8fb4442c7f")
}

/// Snowfall (5ffed544-1656-4753-ac8f-ad8fb4442c7f) — {2}{U} Enchantment:
/// "Cumulative upkeep {U}"
/// (Coverage::Partial — triggered mana ability when an Island is tapped for mana is not supported.)
#[test]
fn snowfall_cumulative_upkeep_scales_with_age_counters_and_sacrifices_when_declined() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[snowfall(), island(), island()])
        .start();
    keep_mulligans(&mut engine);

    let sf = on_battlefield(&engine, P0, snowfall()).unwrap();

    // Upkeep 1 (turn 1): 1 age counter -> pay {U}.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    let isl = on_battlefield(&engine, P0, island()).unwrap();
    tap_mana_where(&mut engine, P0, |id| id == isl);
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    assert_eq!(
        engine
            .state()
            .object(sf)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::AGE),
        1
    );

    // Upkeep 2 (turn 3): 2nd age counter -> pay {U}{U}.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 3 && matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    tap_all_mana(&mut engine, P0);
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.number == 3 && e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    assert_eq!(
        engine
            .state()
            .object(sf)
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
        on_battlefield(&engine, P0, snowfall()).is_none()
            && in_graveyard(&engine, P0, snowfall()).is_some(),
        "Snowfall is sacrificed when cumulative upkeep is declined"
    );
}
