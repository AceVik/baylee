#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn hidden_exile_vault_multiple_activations_sacrifice_only_once_and_return_all_cards() {
    let (mut e, source, first) = vault();
    e.state.set_tapped(source, false);
    e.refresh_offer();
    activate(&mut e, P0, index::KNOWLEDGE_VAULT, 0);
    pass_until(&mut e, stack_is_empty);
    let cards = e.state.zones.list(ZoneLocation::Exile(P0)).clone();
    assert_eq!(cards.len(), 2);
    assert!(cards.contains(&first));
    activate(&mut e, P0, index::KNOWLEDGE_VAULT, 1);
    // Keep priority by explicitly returning to the activation's controller.
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::Priority { player: P0, .. })
    });
    activate(&mut e, P0, index::KNOWLEDGE_VAULT, 1);
    pass_until(&mut e, stack_is_empty);
    assert!(
        cards
            .iter()
            .all(|id| e.state.object(*id).unwrap().zone == Zone::Hand)
    );
}

#[test]
fn hidden_exile_vault_losing_control_prevents_sacrifice_and_hand_discard() {
    let (mut e, source, card) = vault();
    let before = e.state.zones.list(ZoneLocation::Hand(P0)).clone();
    activate(&mut e, P0, index::KNOWLEDGE_VAULT, 1);
    e.state.object_mut(source).unwrap().set_controller(P1);
    e.state.invalidate_projections();
    e.state.refresh_characteristics();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(source).unwrap().zone, Zone::Battlefield);
    assert_eq!(e.state.object(card).unwrap().zone, Zone::Exile);
    assert_eq!(e.state.zones.list(ZoneLocation::Hand(P0)), &before);
    assert!(!e.state.object(card).unwrap().may_look_at_exile(P1));
}

#[test]
fn hidden_exile_vault_leaving_for_any_zone_returns_cards_to_their_owner() {
    for destination in [
        ZoneLocation::Hand(P0),
        ZoneLocation::Library(P0),
        ZoneLocation::Exile(P0),
        ZoneLocation::Graveyard(P0),
    ] {
        let (mut e, source, card) = vault();
        e.state
            .move_object(source, destination, ZonePosition::Top, Cause::Effect)
            .unwrap();
        let player = e.pending().asked().unwrap();
        e.apply(player, PlayerAction::PassPriority).unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state.object(card).unwrap().zone, Zone::Graveyard);
        assert!(
            !e.state
                .object(card)
                .unwrap()
                .status
                .contains(Status::FACE_DOWN)
        );
    }
}

#[test]
fn hidden_exile_conditions_use_signed_power_and_damage_to_one_opponent() {
    let mut e = prepared(&[], &[index::GRIZZLY_BEARS, index::LLANOWAR_ELVES], &[]);
    let bear = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    let elf = on_battlefield(&e, P0, index::LLANOWAR_ELVES).unwrap();
    e.state.object_mut(bear).unwrap().base_mut().power = Some(12);
    e.state.object_mut(elf).unwrap().base_mut().power = Some(-3);
    e.state.invalidate_projections();
    e.state.refresh_characteristics();
    let holds = |e: &Engine<RegistryLookup>, condition| {
        crate::eval::condition_holds(&e.state, P0, bear, condition)
    };
    assert!(!holds(
        &e,
        baylee_cards_dsl::Condition::ControlledCreaturePowerAtLeast(10)
    ));
    e.state.per_turn.damage_dealt_to[0] = 100;
    e.state.per_turn.damage_dealt_to[1] = 6;
    assert!(!holds(
        &e,
        baylee_cards_dsl::Condition::OpponentWasDealtAtLeast(7)
    ));
    e.state.per_turn.damage_dealt_to[1] = 7;
    assert!(holds(
        &e,
        baylee_cards_dsl::Condition::OpponentWasDealtAtLeast(7)
    ));
}
