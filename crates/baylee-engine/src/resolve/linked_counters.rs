//! Counter-bound land changes and the source-incarnation cleanup ledger.
use baylee_cards_dsl::{AbilityDef, CounterKind, Duration, Effect, Layer, Modifier};
use baylee_core::ids::{EffectId, ObjectId};

use super::{AwaitingOp, Resolution};
use crate::choice::{ChoicePrompt, Pending};
use crate::effects::{ContinuousEffect, EffectFilter, EffectOrigin};
use crate::object::Rider;
use crate::state::{DelayedAction, DelayedTrigger, DelayedWhen, GameState};
use crate::zone::Zone;

/// The history belongs to a source incarnation, not to its current card.
#[derive(Clone, Debug, Hash)]
pub struct CounterLink {
    source: ObjectId,
    version: u32,
    kind: CounterKind,
    // Object identity and whether this source has already removed its counters.
    lands: Vec<(ObjectId, u32, bool)>,
}

pub(crate) fn uses_links(abilities: impl Into<crate::copiable_abilities::AbilityDefs>) -> bool {
    abilities.into().iter().any(|ability| match ability {
        AbilityDef::ActivatedConditional { effects, .. }
        | AbilityDef::Activated { effects, .. }
        | AbilityDef::Triggered { effects, .. } => effects.iter().any(|e| {
            matches!(
                e,
                Effect::MarkLandWithCounter { .. } | Effect::ScheduleLinkedCounterCleanup { .. }
            )
        }),
        _ => false,
    })
}

fn source_version(state: &GameState, res: &Resolution) -> u32 {
    state
        .object(res.on_stack)
        .and_then(|o| {
            o.riders.iter().find_map(|r| {
                if let Rider::CounterSourceVersion(version) = r {
                    Some(*version)
                } else {
                    None
                }
            })
        })
        .expect("linked counter ability captured its source incarnation")
}

fn ledger(
    state: &mut GameState,
    source: ObjectId,
    version: u32,
    kind: CounterKind,
) -> &mut CounterLink {
    let index = state
        .counter_links
        .iter()
        .position(|l| l.source == source && l.version == version && l.kind == kind)
        .unwrap_or_else(|| {
            state.counter_links.push(CounterLink {
                source,
                version,
                kind,
                lands: Vec::new(),
            });
            state.counter_links.len() - 1
        });
    &mut state.counter_links[index]
}

pub(super) fn exec(state: &mut GameState, res: &mut Resolution, op: Effect) -> Option<Pending> {
    let version = source_version(state, res);
    match op {
        Effect::MarkLandWithCounter { kind, subtype } => {
            ledger(state, res.source, version, kind);
            for &land in &res.targets {
                let Some(identity) = state
                    .object(land)
                    .filter(|o| o.zone == Zone::Battlefield)
                    .map(|o| o.version)
                else {
                    continue;
                };
                let old = state.object(land).map_or(0, |o| o.counters.get(kind));
                crate::replacement::put_counters(state, land, kind, 1);
                let count = state.object(land).map_or(0, |o| o.counters.get(kind));
                if count > old {
                    let link = ledger(state, res.source, version, kind);
                    if !link
                        .lands
                        .iter()
                        .any(|(id, v, _)| *id == land && *v == identity)
                    {
                        link.lands.push((land, identity, false));
                    }
                }
                // The duration may begin even if placement was prevented but a
                // counter of the same kind was already there.
                if count > 0 {
                    let timestamp = state.next_timestamp();
                    state.effects.register(ContinuousEffect {
                        id: EffectId::new(0),
                        source: Some(res.source),
                        controller: res.controller,
                        origin: EffectOrigin::Resolution,
                        layer: Layer::Type,
                        timestamp,
                        duration: Duration::WhileCounterRemains(kind),
                        filter: EffectFilter::ObjectIs(land, identity),
                        modifier: Modifier::SetLandType(subtype),
                    });
                }
            }
            None
        }
        Effect::ScheduleLinkedCounterCleanup { kind, effects } => {
            ledger(state, res.source, version, kind);
            state.delayed.push(DelayedTrigger {
                controller: res.controller,
                when: DelayedWhen::EachUpkeep,
                action: DelayedAction::LinkedCounterCleanup {
                    source: res.source,
                    version,
                    effects,
                },
            });
            None
        }
        Effect::CleanLinkedCounters { kind } => {
            let options = state
                .counter_links
                .iter()
                .find(|l| l.source == res.source && l.version == version && l.kind == kind)
                .map(|link| {
                    link.lands
                        .iter()
                        .filter_map(|(id, v, removed)| {
                            state
                                .object(*id)
                                .filter(|o| {
                                    !removed
                                        && o.version == *v
                                        && o.zone == Zone::Battlefield
                                        && !o.status.contains(crate::object::Status::PHASED_OUT)
                                        && o.characteristics()
                                            .types
                                            .contains(baylee_core::types::TypeSet::LAND)
                                        && o.counters.get(kind) > 0
                                })
                                .map(|_| *id)
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            if options.is_empty() {
                return None;
            }
            res.awaiting = Some(AwaitingOp::LinkedCounterCleanup { version, kind });
            Some(Pending::ChooseCards {
                player: res.controller,
                options,
                min: 1,
                max: 1,
                prompt: ChoicePrompt::RemoveLandCounters,
                total: None,
            })
        }
        _ => unreachable!("not a linked-counter effect"),
    }
}

pub(super) fn finish_cleanup(
    state: &mut GameState,
    source: ObjectId,
    version: u32,
    kind: CounterKind,
    chosen: &[ObjectId],
) {
    for &land in chosen {
        let removed = crate::replacement::remove_counters(state, land, kind, u16::MAX);
        if removed > 0 {
            let identity = state.object(land).map_or(u32::MAX, |o| o.version);
            for (id, v, cleaned) in &mut ledger(state, source, version, kind).lands {
                if *id == land && *v == identity {
                    *cleaned = true;
                }
            }
        }
    }
}

/// Expiration is permanent, including removal followed by replacement during
/// the same resolution. Phasing does not remove counters or change identity.
pub(crate) fn expire(state: &mut GameState) {
    if !state.effects.may_have_counter_duration() {
        return;
    }
    let expired: Vec<_> = state
        .effects
        .iter()
        .filter_map(|fx| {
            let Duration::WhileCounterRemains(kind) = fx.duration else {
                return None;
            };
            let EffectFilter::ObjectIs(id, version) = fx.filter else {
                return None;
            };
            state
                .object(id)
                .is_none_or(|o| o.version != version || o.counters.get(kind) == 0)
                .then_some(fx.id)
        })
        .collect();
    if !expired.is_empty() {
        state.effects.remove_where(|fx| expired.contains(&fx.id));
    }
}
