//! `cards/artifacts/mv_3/kyren_toy.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn kyren_toy() -> CardIndex {
    card_index("f1a0d112-9e1b-4354-bd7f-0051bff8f904")
}

fn cross_to_next_p0_main(engine: &mut Engine<RegistryLookup>) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    pass_until(engine, |e| e.state().turn.active == p1);
    pass_until(engine, |e| e.state().turn.active == p0);
    reach_main_phase(engine, p0);
}

/// Kyren Toy (f1a0d112-9e1b-4354-bd7f-0051bff8f904) — {3} Artifact:
/// "{1}, {T}: Put a charge counter on this artifact."
/// "{T}, Remove X charge counters from this artifact: Add an amount of {C}
///  equal to X plus one."
///
/// With 0 charge counters, X=0 adds 1 {C} (and X=1 is refused). After charging
/// twice via {1}, {T}, X=2 removes both counters (X=3 refused) and adds 3 {C}.
#[test]
fn kyren_toy_charges_and_adds_x_plus_one_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[kyren_toy(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let toy = on_battlefield(&engine, p0, kyren_toy()).unwrap();

    // 1. With 0 charge counters: X is bounded to 0..=0, and X=0 produces 1 {C}.
    activate(&mut engine, p0, kyren_toy(), 1);
    let Pending::ChooseNumber { min, max, .. } = engine.pending().clone() else {
        panic!("expected ChooseNumber for X, got {:?}", engine.pending())
    };
    assert_eq!((min, max), (0, 0));
    assert!(
        engine.apply(p0, PlayerAction::ChooseNumber(1)).is_err(),
        "cannot remove 1 counter when Toy has 0"
    );
    engine.apply(p0, PlayerAction::ChooseNumber(0)).unwrap();
    assert!(is_tapped(&engine, toy));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1,
        "X=0 adds 0 + 1 = 1 {{C}}"
    );

    // 2. Across two turns, pay {1}, {T} to put 2 charge counters on Kyren Toy.
    for expected in 1..=2u16 {
        cross_to_next_p0_main(&mut engine);
        tap_all_mana(&mut engine, p0);
        activate(&mut engine, p0, kyren_toy(), 0);
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(counters_on(&engine, toy, CounterKind::Charge), expected);
        assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    }

    // 3. Next turn: remove X=2 charge counters -> adds 2 + 1 = 3 {C}.
    cross_to_next_p0_main(&mut engine);
    activate(&mut engine, p0, kyren_toy(), 1);
    let Pending::ChooseNumber { min, max, .. } = engine.pending().clone() else {
        panic!("expected ChooseNumber for X, got {:?}", engine.pending())
    };
    assert_eq!((min, max), (0, 2));
    assert!(
        engine.apply(p0, PlayerAction::ChooseNumber(3)).is_err(),
        "cannot announce X=3 with only 2 charge counters"
    );
    engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
    assert_eq!(counters_on(&engine, toy, CounterKind::Charge), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        3,
        "X=2 adds 2 + 1 = 3 {{C}}"
    );
}
