//! Trade Caravan — `{W}` 1/1 Creature — Human Nomad (Homelands)
//!
//! At the beginning of your upkeep, put a currency counter on this creature.
//! Remove two currency counters from this creature: Untap target basic land. Activate only during
//! an opponent's upkeep.

#[allow(clippy::wildcard_imports)]
use super::*;

fn trade_caravan() -> CardIndex {
    card_index("8faccd56-dfc2-4cfc-880c-db40b935336a")
}

#[test]
fn trade_caravan_gains_currency_counters_and_untaps_basic_land_only_during_opponents_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[trade_caravan(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let caravan = on_battlefield(&engine, p0, trade_caravan()).unwrap();
    let land = on_battlefield(&engine, p0, plains()).unwrap();

    // Turn 1 upkeep already put 1 currency counter on Trade Caravan; pass through one more of p0's upkeeps so it has 2.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(
        engine
            .state()
            .object(caravan)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::CURRENCY),
        2
    );

    // Tap the Plains in p0's main phase; verify Trade Caravan cannot activate during p0's main phase.
    tap_all_mana(&mut engine, p0);
    assert!(is_tapped(&engine, land));
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: caravan,
                    ability_index: 1,
                },
            )
            .is_err(),
        "Trade Caravan cannot activate during its controller's main phase"
    );

    // Advance to p1's upkeep when p0 gets priority: now Trade Caravan can remove 2 currency counters to untap Plains!
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1
            && e.state().turn.step == Step::Upkeep
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: caravan,
                ability_index: 1,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![land],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        !is_tapped(&engine, land),
        "Trade Caravan untapped target basic land during opponent's upkeep"
    );
    assert_eq!(
        engine
            .state()
            .object(caravan)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::CURRENCY),
        0
    );
}

/// "Activate only during an opponent's upkeep": the opponent's main phase is
/// not it, however many counters there are.
#[test]
fn trade_caravan_cannot_untap_a_land_in_an_opponents_main_phase() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[trade_caravan(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let caravan = on_battlefield(&engine, p0, trade_caravan()).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(
        engine
            .state()
            .object(caravan)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::CURRENCY),
        2
    );

    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    assert!(
        !ability_offered(&engine, caravan, 1),
        "two counters, but this is the opponent's main phase and not their upkeep"
    );
}
