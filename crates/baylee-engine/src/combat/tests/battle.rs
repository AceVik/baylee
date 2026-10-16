//! Combat boundaries; entry/defeat are exercised by lifecycle and card tests.

use super::*;
use crate::event::Cause;
use crate::zone::ZonePosition;

const A: PlayerId = PlayerId::new(0);
const B: PlayerId = PlayerId::new(1);
const C: PlayerId = PlayerId::new(2);

fn battle(
    state: &mut GameState,
    controller: PlayerId,
    protector: PlayerId,
    extra: TypeSet,
) -> ObjectId {
    let id = creature(state, controller, 4, 4, KeywordSet::HASTE);
    state.object_mut(id).unwrap().base_mut().types = TypeSet::BATTLE.union(extra);
    let reference = state.source_identity(id).unwrap();
    crate::battle::set_protector(state, reference, protector);
    id
}

fn attack_battle(state: &mut GameState, attacker: ObjectId, battle: ObjectId) {
    let defender = Defender::Battle(battle);
    remember_attack(state, attacker, defender);
    state.combat.declare_attackers([AttackerInfo {
        creature: attacker,
        defending: defender,
        blocked: false,
        band: None,
    }]);
}

#[test]
fn battle_protector_controls_attack_options_and_blocks_not_controller() {
    let mut state = seated(3);
    let siege = battle(&mut state, A, B, TypeSet::EMPTY);
    let attacker = creature(&mut state, A, 2, 2, KeywordSet::HASTE);
    let protector_blocker = creature(&mut state, B, 2, 2, KeywordSet::EMPTY);
    let other_blocker = creature(&mut state, C, 2, 2, KeywordSet::EMPTY);
    assert!(defender_options(&state, A).contains(&Defender::Battle(siege)));
    assert!(!defender_options(&state, B).contains(&Defender::Battle(siege)));
    assert!(defender_options(&state, C).contains(&Defender::Battle(siege)));
    attack_battle(&mut state, attacker, siege);
    assert!(can_block(&state, B, protector_blocker, attacker));
    assert!(!can_block(&state, C, other_blocker, attacker));
    assert_eq!(historical_defending_player(&state, attacker), Some(B));
    assert_eq!(
        active_damage_recipient(&state, attacker),
        Some(DamageTarget::Object(siege))
    );
}

#[test]
fn protector_change_preserves_attackers_blocks_and_original_defending_player() {
    let mut state = seated(3);
    let siege = battle(&mut state, A, B, TypeSet::EMPTY);
    let attacker = creature(&mut state, A, 2, 2, KeywordSet::EMPTY);
    let blocker = creature(&mut state, B, 2, 2, KeywordSet::EMPTY);
    attack_battle(&mut state, attacker, siege);
    state.declare_block(blocker, attacker);
    let reference = state.source_identity(siege).unwrap();
    assert!(state.combat.is_being_attacked(reference));
    crate::battle::set_protector(&mut state, reference, C);
    assert!(state.combat.is_attacking(attacker));
    assert!(state.combat.is_blocked(attacker));
    assert_eq!(state.combat.blockers.len(), 1);
    assert!(!state.combat.is_being_attacked(reference));
    assert_eq!(active_damage_recipient(&state, attacker), None);
    assert_eq!(historical_defending_player(&state, attacker), Some(B));
    assert!(can_block(&state, B, blocker, attacker));
    crate::battle::set_protector(&mut state, reference, B);
    assert_eq!(active_damage_recipient(&state, attacker), None);
}

#[test]
fn blinked_defender_does_not_receive_old_attack_but_new_attacker_can_attack_it() {
    let mut state = empty_state();
    let siege = battle(&mut state, A, B, TypeSet::EMPTY);
    // This blink must complete a real Battle entry transaction. Use a Siege
    // so the two-player protector choice is forced to the opponent.
    let base = state.object_mut(siege).unwrap().base_mut();
    base.defense = Some(4);
    base.subtypes
        .insert(baylee_core::generated::subtypes::battle::SIEGE);
    let old = creature(&mut state, A, 2, 2, KeywordSet::EMPTY);
    attack_battle(&mut state, old, siege);
    state
        .move_object(
            siege,
            ZoneLocation::Exile(A),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    state
        .move_object(
            siege,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    assert!(crate::entry::advance(&mut state).is_none());
    assert_eq!(
        state.object(siege).unwrap().zone,
        crate::zone::Zone::Battlefield
    );
    assert_eq!(crate::battle::protector(&state, siege), Some(B));
    let new = creature(&mut state, A, 3, 3, KeywordSet::EMPTY);
    attack_battle(&mut state, new, siege);
    assert!(state.combat.is_attacking(old));
    assert_eq!(active_damage_recipient(&state, old), None);
    assert_eq!(
        active_damage_recipient(&state, new),
        Some(DamageTarget::Object(siege))
    );
    assert_eq!(historical_defending_player(&state, old), Some(B));
    let damage = collect_combat_damage(&mut state, false);
    assert!(!damage.iter().any(|part| part.source == old));
    assert!(
        damage
            .iter()
            .any(|part| part.source == new && part.recipient == DamageTarget::Object(siege))
    );
}

#[test]
fn battle_planeswalker_type_loss_uses_the_hybrid_exception() {
    for (controller, remaining, stays) in [
        (A, TypeSet::BATTLE, true),
        (A, TypeSet::PLANESWALKER, false),
        (B, TypeSet::PLANESWALKER, true),
        (B, TypeSet::ARTIFACT, false),
    ] {
        let mut state = empty_state();
        let target = battle(&mut state, controller, B, TypeSet::PLANESWALKER);
        let attacker = creature(&mut state, A, 2, 2, KeywordSet::EMPTY);
        attack_battle(&mut state, attacker, target);
        assert_eq!(defender_options(&state, A).iter().filter(|d| matches!(d, Defender::Battle(id) | Defender::Planeswalker(id) if *id == target)).count(), 1);
        state.object_mut(target).unwrap().base_mut().types = remaining;
        state.refresh_characteristics();
        recheck_defenders(&mut state);
        assert_eq!(active_damage_recipient(&state, attacker).is_some(), stays);
        assert!(state.combat.is_attacking(attacker));
    }
}

#[test]
fn an_animated_battle_cannot_attack_or_block_and_becoming_one_removes_a_combatant() {
    let mut state = empty_state();
    let animated = battle(&mut state, A, B, TypeSet::CREATURE);
    let attacker = creature(&mut state, B, 2, 2, KeywordSet::HASTE);
    assert!(!can_attack(&state, A, animated));
    attack(&mut state, attacker, A);
    assert!(!can_block(&state, A, animated, attacker));
    let ordinary = creature(&mut state, A, 2, 2, KeywordSet::HASTE);
    state.declare_block(ordinary, attacker);
    state.object_mut(ordinary).unwrap().base_mut().types = TypeSet::CREATURE.union(TypeSet::BATTLE);
    state.refresh_characteristics();
    recheck_defenders(&mut state);
    assert!(state.combat.blockers.is_empty());
    assert!(state.combat.is_blocked(attacker));
}

#[test]
fn battle_type_loss_then_restoration_does_not_reactivate_an_attack() {
    let mut state = empty_state();
    let target = battle(&mut state, A, B, TypeSet::EMPTY);
    let attacker = creature(&mut state, A, 2, 2, KeywordSet::EMPTY);
    attack_battle(&mut state, attacker, target);
    state.object_mut(target).unwrap().base_mut().types = TypeSet::ARTIFACT;
    recheck_defenders(&mut state);
    state.object_mut(target).unwrap().base_mut().types = TypeSet::BATTLE;
    recheck_defenders(&mut state);
    assert_eq!(active_damage_recipient(&state, attacker), None);
    assert!(state.combat.is_attacking(attacker));
}

#[test]
fn phasing_and_control_change_end_defender_combat_without_changing_history() {
    for phase in [false, true] {
        let mut state = seated(3);
        let target = battle(&mut state, A, B, TypeSet::EMPTY);
        let attacker = creature(&mut state, A, 2, 2, KeywordSet::EMPTY);
        let blocker = creature(&mut state, B, 2, 2, KeywordSet::EMPTY);
        attack_battle(&mut state, attacker, target);
        if phase {
            state.phase_out(&[target]);
            state.phase_in(target);
        } else {
            state.object_mut(target).unwrap().base_controller = C;
            state.refresh_characteristics();
            state.object_mut(target).unwrap().base_controller = A;
            state.refresh_characteristics();
        }
        assert!(state.combat.is_attacking(attacker));
        assert_eq!(active_damage_recipient(&state, attacker), None);
        assert_eq!(historical_defending_player(&state, attacker), Some(B));
        assert!(can_block(&state, B, blocker, attacker));
    }
}

#[test]
fn unchanged_protector_does_not_remove_defender_from_combat() {
    let mut state = empty_state();
    let target = battle(&mut state, A, B, TypeSet::EMPTY);
    let attacker = creature(&mut state, A, 2, 2, KeywordSet::EMPTY);
    attack_battle(&mut state, attacker, target);
    let reference = state.source_identity(target).unwrap();
    crate::battle::set_protector(&mut state, reference, B);
    assert!(state.combat.is_being_attacked(reference));
    assert_eq!(
        active_damage_recipient(&state, attacker),
        Some(DamageTarget::Object(target))
    );
}

#[test]
fn planeswalker_gaining_battle_then_losing_planeswalker_remains_attacked() {
    let mut state = empty_state();
    let target = creature(&mut state, B, 1, 1, KeywordSet::EMPTY);
    state.object_mut(target).unwrap().base_mut().types = TypeSet::PLANESWALKER;
    let attacker = creature(&mut state, A, 2, 2, KeywordSet::EMPTY);
    let defender = Defender::Planeswalker(target);
    remember_attack(&mut state, attacker, defender);
    state.combat.declare_attackers([AttackerInfo {
        creature: attacker,
        defending: defender,
        blocked: false,
        band: None,
    }]);
    let reference = state.source_identity(target).unwrap();
    state.object_mut(target).unwrap().base_mut().types =
        TypeSet::PLANESWALKER.union(TypeSet::BATTLE);
    state.refresh_characteristics();
    recheck_defenders(&mut state);
    assert!(state.combat.is_being_attacked(reference));
    assert_eq!(crate::battle::designated_protector(&state, target), None);
    let blocker = creature(&mut state, B, 2, 2, KeywordSet::EMPTY);
    assert_eq!(historical_defending_player(&state, attacker), None);
    assert!(!can_block(&state, B, blocker, attacker));
    state.object_mut(target).unwrap().base_mut().types = TypeSet::BATTLE;
    state.refresh_characteristics();
    recheck_defenders(&mut state);
    assert!(state.combat.is_being_attacked(reference));
    assert_eq!(
        active_damage_recipient(&state, attacker),
        Some(DamageTarget::Object(target))
    );
    state.combat.remove_defender_from_combat(reference);
    assert_eq!(historical_defending_player(&state, attacker), None);
    assert!(!can_block(&state, B, blocker, attacker));
}

#[test]
fn battle_gaining_planeswalker_uses_new_hybrid_status_when_losing_battle() {
    for (controller, stays) in [(A, false), (B, true)] {
        let mut state = empty_state();
        let target = battle(&mut state, controller, B, TypeSet::EMPTY);
        let attacker = creature(&mut state, A, 2, 2, KeywordSet::EMPTY);
        attack_battle(&mut state, attacker, target);
        state.object_mut(target).unwrap().base_mut().types =
            TypeSet::BATTLE.union(TypeSet::PLANESWALKER);
        state.refresh_characteristics();
        recheck_defenders(&mut state);
        state.object_mut(target).unwrap().base_mut().types = TypeSet::PLANESWALKER;
        state.refresh_characteristics();
        recheck_defenders(&mut state);
        assert_eq!(active_damage_recipient(&state, attacker).is_some(), stays);
        assert!(state.combat.is_attacking(attacker));
    }
}
