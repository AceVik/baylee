//! Battle damage results use the same atomic damage event and counter journal.
use super::*;
use crate::event::{Cause, GameEvent};
use crate::zone::ZonePosition;
use baylee_cards_dsl::{Filter, Modifier, ReplacementRule};

fn battle(state: &mut GameState, types: TypeSet, defense: u16) -> ObjectId {
    let target = creature(state, A, 8, 8);
    state.object_mut(target).unwrap().base_mut().types = types;
    crate::replacement::put_counters(state, target, CounterKind::Defense, defense);
    crate::replacement::put_counters(state, target, CounterKind::Loyalty, 5);
    target
}

#[test]
fn every_hybrid_receives_independent_results_without_capping_damage_to_defense() {
    for creature_type in [false, true] {
        for planeswalker_type in [false, true] {
            for infect in [false, true] {
                for amount in [0, 1, 3, 7, 70_000] {
                    let mut state = state(2);
                    let source = creature(&mut state, B, 8, 8);
                    state.object_mut(source).unwrap().base_mut().keywords = if infect {
                        KeywordSet::LIFELINK.union(KeywordSet::INFECT)
                    } else {
                        KeywordSet::LIFELINK
                    };
                    let mut types = TypeSet::BATTLE;
                    if creature_type {
                        types = types.union(TypeSet::CREATURE);
                    }
                    if planeswalker_type {
                        types = types.union(TypeSet::PLANESWALKER);
                    }
                    let target = battle(&mut state, types, 3);
                    let version = state.object(target).unwrap().version;
                    let start = state.journal.last_seq();
                    let mut work = DamageWork::new(
                        &mut state,
                        vec![assigned(source, DamageTarget::Object(target), amount)],
                    );
                    assert!(work.advance(&mut state).is_none());
                    let object = state.object(target).unwrap();
                    let bounded = u16::try_from(amount).unwrap_or(u16::MAX);
                    assert_eq!(
                        object.counters.get(CounterKind::Defense),
                        3u16.saturating_sub(bounded)
                    );
                    assert_eq!(
                        object.counters.get(CounterKind::Loyalty),
                        if planeswalker_type {
                            5u16.saturating_sub(bounded)
                        } else {
                            5
                        }
                    );
                    assert_eq!(
                        object.damage,
                        if creature_type && !infect { bounded } else { 0 }
                    );
                    assert_eq!(
                        object.counters.get(CounterKind::M1M1),
                        if creature_type && infect { bounded } else { 0 }
                    );
                    assert_eq!(state.players[1].life, 20 + i32::try_from(amount).unwrap());
                    assert_eq!(work.dealt(), amount);
                    let events: Vec<_> = state
                        .journal
                        .entries()
                        .iter()
                        .filter(|entry| entry.seq > start)
                        .collect();
                    if amount > 0 {
                        assert!(events.iter().any(|entry| matches!(entry.event, GameEvent::CounterChanged { object, version: changed_version, kind: CounterKind::Defense, old: 3, new, .. } if object == target && changed_version == version && new == 3u16.saturating_sub(bounded))));
                        assert!(events.iter().any(|entry| matches!(entry.event, GameEvent::DamageDealt { source: Some(id), target: DamageTarget::Object(to), amount: dealt, .. } if id == source && to == target && dealt == amount)));
                    } else {
                        assert!(events.is_empty());
                    }
                }
            }
        }
    }
}

#[test]
fn prevention_controls_defense_loss_and_lifelink_but_cannot_limit_unpreventable_damage() {
    for prevent in [2, 8] {
        for unpreventable in [false, true] {
            let mut state = state(2);
            let source = creature(&mut state, B, 5, 5);
            state.object_mut(source).unwrap().base_mut().keywords = KeywordSet::LIFELINK;
            let target = battle(&mut state, TypeSet::BATTLE, 4);
            let version = state.object(target).unwrap().version;
            state.shields.push(Shield {
                protects: Shielded::Object(target, version),
                kind: ShieldKind::Next(prevent),
                controller: A,
            });
            if unpreventable {
                replacement(&mut state, source, Modifier::CombatDamageCantBePrevented);
            }
            let mut assignment = assigned(source, DamageTarget::Object(target), 5);
            assignment.is_combat = true;
            let mut work = DamageWork::new(&mut state, vec![assignment]);
            assert!(work.advance(&mut state).is_none());
            let dealt = if unpreventable {
                5
            } else {
                5u32.saturating_sub(prevent)
            };
            assert_eq!(work.dealt(), dealt);
            assert_eq!(
                state
                    .object(target)
                    .unwrap()
                    .counters
                    .get(CounterKind::Defense),
                4u16.saturating_sub(u16::try_from(dealt).unwrap())
            );
            assert_eq!(state.object(target).unwrap().damage, 0);
            assert_eq!(state.players[1].life, 20 + i32::try_from(dealt).unwrap());
        }
    }
}

#[test]
fn infect_ordering_holds_defense_loyalty_and_lifelink_until_all_results_commit() {
    let mut state = state(2);
    let source = creature(&mut state, B, 2, 2);
    state.object_mut(source).unwrap().base_mut().keywords =
        KeywordSet::LIFELINK.union(KeywordSet::INFECT);
    let target = battle(
        &mut state,
        TypeSet::BATTLE
            .union(TypeSet::CREATURE)
            .union(TypeSet::PLANESWALKER),
        4,
    );
    replacement(
        &mut state,
        source,
        Modifier::AdditionalCounterPlacement { n: 1 },
    );
    state
        .replacement_rules
        .push(crate::state::ReplacementEntry {
            source,
            controller: A,
            rule: ReplacementRule::DoubleCounterPlacement {
                object_filter: &Filter::Any,
            },
        });
    let before = state.journal.last_seq();
    let mut work = DamageWork::new(
        &mut state,
        vec![assigned(source, DamageTarget::Object(target), 2)],
    );
    assert!(matches!(
        work.advance(&mut state),
        Some(Pending::YesNo { .. })
    ));
    assert_eq!(state.journal.last_seq(), before);
    let object = state.object(target).unwrap();
    assert_eq!(object.counters.get(CounterKind::Defense), 4);
    assert_eq!(object.counters.get(CounterKind::Loyalty), 5);
    assert_eq!(object.counters.get(CounterKind::M1M1), 0);
    assert_eq!(state.players[1].life, 20);
    let mut checkpoint = state.clone();
    let mut saved_work = work.clone();
    assert!(
        work.answer(&mut state, &PlayerAction::YesNo(true))
            .is_none()
    );
    assert!(
        saved_work
            .answer(&mut checkpoint, &PlayerAction::YesNo(true))
            .is_none()
    );
    assert_eq!(state.snapshot_hash(), checkpoint.snapshot_hash());
    let object = state.object(target).unwrap();
    assert_eq!(object.counters.get(CounterKind::Defense), 2);
    assert_eq!(object.counters.get(CounterKind::Loyalty), 3);
    assert_eq!(object.counters.get(CounterKind::M1M1), 6);
    assert_eq!(object.damage, 0);
    assert_eq!(state.players[1].life, 22);
}

#[test]
fn old_source_lifelink_survives_departure_but_old_recipient_does_not_follow_blink() {
    let mut state = state(2);
    let source = creature(&mut state, B, 2, 2);
    state.object_mut(source).unwrap().base_mut().keywords = KeywordSet::LIFELINK;
    state.refresh_characteristics();
    let source_version = state.object(source).unwrap().version;
    let target = battle(&mut state, TypeSet::BATTLE, 4);
    state
        .move_object(
            source,
            ZoneLocation::Exile(B),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    state
        .move_object(
            source,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    state.object_mut(source).unwrap().base_mut().keywords = KeywordSet::EMPTY;
    let mut assignment = assigned(source, DamageTarget::Object(target), 2);
    assignment.source_version = Some(source_version);
    let mut work = DamageWork::new(&mut state, vec![assignment]);
    assert!(work.advance(&mut state).is_none());
    assert_eq!(state.players[1].life, 22);
    assert_eq!(
        state
            .object(target)
            .unwrap()
            .counters
            .get(CounterKind::Defense),
        2
    );
    let mut stale = DamageWork::new(&mut state, vec![assignment]);
    state
        .move_object(
            target,
            ZoneLocation::Exile(A),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    state
        .move_object(
            target,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    crate::replacement::put_counters(&mut state, target, CounterKind::Defense, 4);
    assert!(stale.advance(&mut state).is_none());
    assert_eq!(stale.dealt(), 0);
    assert_eq!(
        state
            .object(target)
            .unwrap()
            .counters
            .get(CounterKind::Defense),
        4
    );
    assert_eq!(state.players[1].life, 22);
}
