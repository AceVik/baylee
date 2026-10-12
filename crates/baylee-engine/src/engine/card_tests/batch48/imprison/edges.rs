//! Child module of `batch48_imprison`; actual Implemented card actions.
#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn copied_imprison_trigger_keeps_original_activated_ability_identity() {
    let mut e = prepared(
        &[index::IMPRISON, index::VIRTUE_OF_KNOWLEDGE],
        &[],
        &[index::PRODIGAL_SORCERER],
    );
    let sorcerer = source(&e, P1, index::PRODIGAL_SORCERER);
    cast(&mut e, index::IMPRISON);
    targets(&mut e, vec![sorcerer], vec![]);
    pass_until(&mut e, stack_is_empty);
    let aura = source(&e, P0, index::IMPRISON);
    let before = e.state.players[0].life;
    ability(&mut e, P1, index::PRODIGAL_SORCERER, 0);
    targets(&mut e, vec![], vec![P0]);
    priority(&mut e, P0);
    let trigger = *e
        .state
        .zones
        .list(ZoneLocation::Stack)
        .iter()
        .find(|&&id| {
            e.state
                .object(id)
                .is_some_and(|o| o.ability.is_some_and(|loc| loc.source == aura))
        })
        .expect("actual Imprison trigger");
    cast(&mut e, index::VIRTUE_OF_KNOWLEDGE);
    if let Pending::ChooseCastMode { options, .. } = e.pending() {
        let mode = options
            .iter()
            .position(|o| matches!(o.kind, crate::choice::CastModeKind::Face(1)))
            .unwrap();
        e.apply(P0, PlayerAction::ChooseMode(mode)).unwrap();
    }
    targets(&mut e, vec![trigger], vec![]);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    imprison_payment(&mut e, true);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    imprison_payment(&mut e, false);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state.players[0].life, before,
        "copied trigger counters original activation even after original trigger destroys Aura"
    );
    assert!(on_battlefield(&e, P0, index::IMPRISON).is_none());
}

#[test]
fn imprison_already_triggered_activation_counter_survives_actual_aura_departure() {
    let mut e = prepared(
        &[index::IMPRISON, index::DISENCHANT],
        &[],
        &[index::PRODIGAL_SORCERER],
    );
    let sorcerer = source(&e, P1, index::PRODIGAL_SORCERER);
    cast(&mut e, index::IMPRISON);
    targets(&mut e, vec![sorcerer], vec![]);
    pass_until(&mut e, stack_is_empty);
    let aura = source(&e, P0, index::IMPRISON);
    let before = e.state.players[0].life;
    ability(&mut e, P1, index::PRODIGAL_SORCERER, 0);
    targets(&mut e, vec![], vec![P0]);
    cast(&mut e, index::DISENCHANT);
    targets(&mut e, vec![aura], vec![]);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    assert_eq!(
        e.state.object(aura).unwrap().zone,
        crate::zone::Zone::Graveyard
    );
    imprison_payment(&mut e, true);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state.players[0].life, before,
        "trigger's captured stack identity survives Aura departure"
    );
}

#[test]
fn imprison_payment_uses_actual_open_mana_window_and_real_sol_ring_activation() {
    let mut e = prepared(
        &[index::IMPRISON],
        &[index::SOL_RING],
        &[index::PRODIGAL_SORCERER],
    );
    let sorcerer = source(&e, P1, index::PRODIGAL_SORCERER);
    let ring = source(&e, P0, index::SOL_RING);
    cast(&mut e, index::IMPRISON);
    targets(&mut e, vec![sorcerer], vec![]);
    pass_until(&mut e, stack_is_empty);
    let before = e.state.players[0].life;
    ability(&mut e, P1, index::PRODIGAL_SORCERER, 0);
    targets(&mut e, vec![], vec![P0]);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    // Exact empty-pool boundary fixture; actual mana ability funds the price.
    e.state.players[0].mana_pool = baylee_core::mana::ManaPool::new();
    e.apply(P0, PlayerAction::YesNo(true)).unwrap();
    assert!(e.mana_window.is_some());
    assert!(matches!(e.pending(),Pending::Priority{player,..}if *player==P0));
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: ring,
            ability_index: 0,
        },
    )
    .unwrap();
    assert!(is_tapped(&e, ring));
    e.apply(P0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.players[0].mana_pool.total(), 1);
    assert_eq!(e.state.players[0].life, before);
}

#[test]
fn imprison_combat_trigger_does_not_tap_returned_creature_incarnation() {
    let mut e = prepared(
        &[index::IMPRISON, index::EPHEMERATE],
        &[index::GRIZZLY_BEARS],
        &[],
    );
    let bear = source(&e, P0, index::GRIZZLY_BEARS);
    cast(&mut e, index::IMPRISON);
    targets(&mut e, vec![bear], vec![]);
    pass_until(&mut e, stack_is_empty);
    let version = e.state.object(bear).unwrap().version;
    attackers(&mut e, &[bear]);
    cast(&mut e, index::EPHEMERATE);
    targets(&mut e, vec![bear], vec![]);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    assert_ne!(e.state.object(bear).unwrap().version, version);
    assert!(!is_tapped(&e, bear));
    assert!(
        on_battlefield(&e, P0, index::IMPRISON).is_none(),
        "old Aura falls off on blink"
    );
    imprison_payment(&mut e, true);
    pass_until(&mut e, stack_is_empty);
    assert!(!e.state.combat.is_attacking(bear));
    assert!(
        !is_tapped(&e, bear),
        "event object identity must not act on new incarnation"
    );
}
