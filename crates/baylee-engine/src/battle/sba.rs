//! Simultaneous battle state-based actions and staged protector decisions.
use crate::object::{GameObject, ObjectKind, Rider};
use crate::state::{CardLookup, GameState};
use crate::zone::{Zone, ZoneLocation};
use baylee_cards_dsl::{AbilityDef, CounterKind};
use baylee_core::ids::{AbilityRef, DamageSourceRef, PlayerId};
use baylee_core::types::TypeSet;

/// One answer retained until every decision for the same SBA event is made.
#[derive(Clone, Debug, Hash)]
pub(crate) struct ProtectorDecision {
    battle: DamageSourceRef,
    controller: PlayerId,
    options: Vec<PlayerId>,
    chosen: PlayerId,
}

pub(crate) type ProtectorChoice = (PlayerId, DamageSourceRef, Vec<PlayerId>);

/// Candidates observed before any of this pass's permanent movements.
#[derive(Default)]
pub(crate) struct Plan {
    pub graveyard: Vec<DamageSourceRef>,
    pub choice: Option<ProtectorChoice>,
    protectors: Vec<(DamageSourceRef, PlayerId)>,
}

fn repair_options(state: &GameState, battle: &GameObject) -> Option<Vec<PlayerId>> {
    let reference = DamageSourceRef {
        object: battle.id,
        version: battle.version,
    };
    let protector = super::designated_protector(state, battle.id);
    let missing = protector.is_none_or(|player| state.has_left(player));
    if let Some(player) = protector.filter(|_| !missing)
        && if super::is_siege(battle) {
            state.is_opponent(player, battle.controller)
        } else {
            player == battle.controller
        }
    {
        return None; // Stable designations need no eligibility-vector allocation.
    }
    // CR 704.5x waits for attackers to leave; 704.5y has no such gate.
    if missing && state.combat.is_being_attacked(reference) {
        return None;
    }
    let options = super::eligible_protectors(state, battle);
    (missing || protector.is_some_and(|player| !options.contains(&player))).then_some(options)
}

/// Exact-source triggered abilities hold a Siege; activated abilities do not.
fn has_stacked_trigger(
    state: &GameState,
    lookup: &impl CardLookup,
    source: DamageSourceRef,
) -> bool {
    state
        .zones
        .list(ZoneLocation::Stack)
        .iter()
        .filter_map(|id| state.object(*id))
        .any(|object| {
            let Some(location) = object.ability else {
                return false;
            };
            if object.kind != ObjectKind::AbilityOnStack
                || location.source != source.object
                || !object
                    .riders
                    .contains(&Rider::AbilitySourceVersion(source.version))
                || object.riders.contains(&Rider::GrantedActivated)
                // A copy has the same source, but did not itself trigger.
                || object.riders.contains(&Rider::AbilityCopy)
            {
                return false;
            }
            location.index == AbilityRef::SYNTHETIC
                || matches!(
                    object
                        .printed_abilities(lookup)
                        .get(location.index as usize),
                    Some(
                        AbilityDef::Triggered { .. }
                            | AbilityDef::ModalTriggered { .. }
                            | AbilityDef::SagaChapter { .. }
                    )
                )
        })
}

pub(crate) fn plan(
    state: &GameState,
    lookup: &impl CardLookup,
    queued_sources: &[DamageSourceRef],
) -> Plan {
    let mut plan = Plan::default();
    for id in state.battlefield_seen() {
        let Some(object) = state.object(id) else {
            continue;
        };
        if object.kind != ObjectKind::Permanent
            || !object.characteristics().types.contains(TypeSet::BATTLE)
        {
            continue;
        }
        let reference = DamageSourceRef {
            object: id,
            version: object.version,
        };
        if object.counters.get(CounterKind::Defense) == 0
            && (!super::is_siege(object)
                || !(queued_sources.contains(&reference)
                    || has_stacked_trigger(state, lookup, reference)))
        {
            plan.graveyard.push(reference);
        }
        let Some(options) = repair_options(state, object) else {
            continue;
        };
        if options.is_empty() {
            if !plan.graveyard.contains(&reference) {
                plan.graveyard.push(reference);
            }
            continue;
        }
        let chosen = if options.len() == 1 {
            Some(options[0])
        } else {
            state
                .battle_sba_decisions
                .iter()
                .find(|decision| {
                    decision.battle == reference
                        && decision.controller == object.controller
                        && decision.options == options
                })
                .map(|decision| decision.chosen)
        };
        if let Some(chosen) = chosen {
            plan.protectors.push((reference, chosen));
        } else if plan.choice.as_ref().is_none_or(|(player, _, _)| {
            apnap_distance(state, object.controller) < apnap_distance(state, *player)
        }) {
            plan.choice = Some((object.controller, reference, options));
        }
    }
    plan
}

pub(crate) fn apnap_distance(state: &GameState, player: PlayerId) -> usize {
    (usize::from(player.get()) + state.players.len() - usize::from(state.turn.active.get()))
        % state.players.len()
}

/// A valid answer records intent only: no protector changes before the full pass.
pub(crate) fn answer(
    state: &mut GameState,
    battle: DamageSourceRef,
    player: PlayerId,
    chosen: PlayerId,
) -> bool {
    let Some(object) = state.source_object(battle).filter(|object| {
        object.zone == Zone::Battlefield
            && object.controller == player
            && object.characteristics().types.contains(TypeSet::BATTLE)
    }) else {
        return false;
    };
    // source_object can read historical objects; a choice must name the live incarnation.
    if state.source_identity(battle.object) != Some(battle) {
        return false;
    }
    let Some(options) = repair_options(state, object) else {
        return false;
    };
    if !options.contains(&chosen) {
        return false;
    }
    state
        .battle_sba_decisions
        .retain(|decision| decision.battle != battle);
    state.battle_sba_decisions.push(ProtectorDecision {
        battle,
        controller: player,
        options,
        chosen,
    });
    true
}

/// Apply choices alongside all other SBAs, skipping objects that left in this pass.
pub(crate) fn apply_protectors(state: &mut GameState, plan: Plan) -> bool {
    state.battle_sba_decisions.clear();
    let mut changed = false;
    for (reference, chosen) in plan.protectors {
        if state.source_identity(reference.object) == Some(reference)
            && state
                .object(reference.object)
                .is_some_and(|object| object.zone == Zone::Battlefield)
        {
            super::set_protector(state, reference, chosen);
            changed = true;
        }
    }
    changed
}
