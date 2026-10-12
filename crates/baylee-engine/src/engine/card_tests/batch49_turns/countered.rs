//! Countered turn abilities preserve paid costs and numeric failure rolls back.
#[allow(clippy::wildcard_imports)]
use super::*;

fn counter_magosi_with_tidebinder(e: &mut Engine<RegistryLookup>) {
    let ability = *e.state.zones.list(ZoneLocation::Stack).last().unwrap();
    turn_priority(e, P0);
    turn_mana(e, P0);
    let card = in_hand(e, P0, index::TISHANA_S_TIDEBINDER).unwrap();
    e.apply(P0, PlayerAction::CastSpell { card }).unwrap();
    pass_until(e, |e| matches!(e.pending(), Pending::ChooseTargets { .. }));
    targets(e, vec![ability]);
    pass_until(e, stack_is_empty);
}

#[test]
fn countered_magosi_counter_and_skip_ability_keeps_costs_without_creating_skip() {
    let mut e = prepared(
        &[index::TISHANA_S_TIDEBINDER],
        &[index::MAGOSI_THE_WATERVEIL],
        &[],
    );
    let land = on_battlefield(&e, P0, index::MAGOSI_THE_WATERVEIL).unwrap();
    e.state.set_tapped(land, false);
    turn_mana(&mut e, P0);
    let blue = e.state.players[0].mana_pool.available(ManaColor::Blue);
    activate(&mut e, P0, index::MAGOSI_THE_WATERVEIL, 1);
    assert!(is_tapped(&e, land));
    assert_eq!(
        e.state.players[0].mana_pool.available(ManaColor::Blue),
        blue - 1
    );
    counter_magosi_with_tidebinder(&mut e);
    assert!(is_tapped(&e, land));
    assert_eq!(
        e.state
            .object(land)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::EON),
        0
    );
    let mut no = false;
    assert_eq!(next_actual_main(&mut e, &mut no), P1);
    assert_eq!(
        next_actual_main(&mut e, &mut no),
        P0,
        "countered skip has no turn obligation"
    );
}

#[test]
fn countered_magosi_extra_turn_keeps_return_and_counter_removal_costs() {
    let mut e = prepared(
        &[index::TISHANA_S_TIDEBINDER],
        &[index::MAGOSI_THE_WATERVEIL],
        &[],
    );
    let land = on_battlefield(&e, P0, index::MAGOSI_THE_WATERVEIL).unwrap();
    e.state.set_tapped(land, false);
    magosi_skip(&mut e);
    // Consume the actual owed skip before testing whether the extra turn was countered.
    let mut no = false;
    assert_eq!(next_actual_main(&mut e, &mut no), P1);
    assert_eq!(next_actual_main(&mut e, &mut no), P1);
    assert_eq!(next_actual_main(&mut e, &mut no), P0);
    assert_eq!(
        e.state
            .object(land)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::EON),
        1
    );
    activate(&mut e, P0, index::MAGOSI_THE_WATERVEIL, 2);
    assert_eq!(e.state.object(land).unwrap().zone, Zone::Hand);
    assert_eq!(
        e.state
            .object(land)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::EON),
        0
    );
    counter_magosi_with_tidebinder(&mut e);
    assert_eq!(e.state.object(land).unwrap().zone, Zone::Hand);
    assert_eq!(
        next_actual_main(&mut e, &mut no),
        P1,
        "countered extra turn creates no queued P0 turn"
    );
}

#[test]
fn magosi_turn_obligation_overflow_reports_numeric_failure_without_wrapping() {
    let mut e = prepared(&[], &[index::MAGOSI_THE_WATERVEIL], &[]);
    let land = on_battlefield(&e, P0, index::MAGOSI_THE_WATERVEIL).unwrap();
    e.state.set_tapped(land, false);
    // Transparent numeric boundary fixture; the overflowing increment is a real card resolution.
    e.state.players[0].turns_to_skip = u32::MAX;
    turn_mana(&mut e, P0);
    activate(&mut e, P0, index::MAGOSI_THE_WATERVEIL, 1);
    let mut overflow = false;
    for _ in 0..8 {
        let Pending::Priority { player, .. } = e.pending() else {
            panic!("resolution requires only priority passes")
        };
        let player = *player;
        let before = e.snapshot_hash();
        match e.apply(player, PlayerAction::PassPriority) {
            Err(crate::engine::EngineError::NumericCapacity(
                "turn skip obligations exceed u32",
            )) => {
                assert_eq!(
                    e.snapshot_hash(),
                    before,
                    "failed resolution pass is transactional"
                );
                overflow = true;
                break;
            }
            Ok(()) => {}
            other => panic!("unexpected result: {other:?}"),
        }
    }
    assert!(
        overflow,
        "actual resolution must report its numeric capacity failure"
    );
    assert_eq!(e.state.players[0].turns_to_skip, u32::MAX);
    assert_eq!(
        e.state.numeric_failure, None,
        "rollback clears the transient failure marker"
    );
}
