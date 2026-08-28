//! Triggered abilities: collection and APNAP ordering.
//!
//! After every mutation batch the engine scans new journal entries and
//! matches them against the triggered abilities of all permanents (and,
//! later, cards in other zones). Matches are put on the stack in APNAP
//! order. Same-controller ordering currently follows timestamp; the
//! player-facing ordering choice is M2 (documented in engine-internals).

use crate::eval;
use crate::event::GameEvent;
use crate::state::{CardLookup, GameState};
use crate::zone::{Zone, ZoneLocation};
use baylee_cards_dsl::{AbilityDef, PlayerRel, StepKind, Trigger};
use baylee_core::ids::{ObjectId, PlayerId};

/// A triggered ability waiting to go on the stack.
#[derive(Clone, Copy, Debug)]
pub struct PendingTrigger {
    /// The permanent whose ability triggered.
    pub source: ObjectId,
    /// Index into the source card's abilities.
    pub ability_index: u32,
    /// Controller of the trigger.
    pub controller: PlayerId,
    /// Timestamp of the source (stable same-controller ordering).
    pub timestamp: u64,
}

/// Matches new journal entries (from `from_seq` onward) against all
/// triggered abilities on the battlefield.
#[must_use]
pub fn collect(state: &GameState, lookup: &impl CardLookup, from_seq: u64) -> Vec<PendingTrigger> {
    let events = &state.journal.entries()[from_seq as usize..];
    if events.is_empty() {
        return Vec::new();
    }
    let mut triggers = Vec::new();
    for &permanent in state.zones.list(ZoneLocation::Battlefield) {
        let Some(obj) = state.object(permanent) else {
            continue;
        };
        let Some(card) = obj.card else { continue };
        let Some(def) = lookup.card(card.index) else {
            continue;
        };
        for (index, ability) in def.abilities.iter().enumerate() {
            let AbilityDef::Triggered { trigger, .. } = ability else {
                continue;
            };
            for entry in events {
                if matches(trigger, &entry.event, state, permanent, obj.controller) {
                    triggers.push(PendingTrigger {
                        source: permanent,
                        ability_index: index as u32,
                        controller: obj.controller,
                        timestamp: obj.timestamp,
                    });
                    break; // one trigger per event per ability — next event
                }
            }
        }
    }
    // APNAP: active player first, then in turn order; same controller by
    // timestamp (M2: player ordering choice).
    let active = state.turn.active;
    triggers.sort_by_key(|t| {
        let distance = (t.controller.get() + state.players.len() as u8 - active.get())
            % state.players.len() as u8;
        (distance, t.timestamp)
    });
    triggers
}

fn matches(
    trigger: &Trigger,
    event: &GameEvent,
    state: &GameState,
    source: ObjectId,
    you: PlayerId,
) -> bool {
    match (trigger, event) {
        (
            Trigger::EntersBattlefield(filter),
            GameEvent::ZoneChanged {
                object,
                to: Zone::Battlefield,
                ..
            },
        ) => state
            .object(*object)
            .is_some_and(|o| eval::matches(filter, state, o, you, source)),
        (
            Trigger::LeavesBattlefield(filter),
            GameEvent::ZoneChanged {
                object,
                from: Zone::Battlefield,
                ..
            },
        ) => state
            .object(*object)
            .is_some_and(|o| eval::matches(filter, state, o, you, source)),
        (
            Trigger::Dies(filter),
            GameEvent::ZoneChanged {
                object,
                from: Zone::Battlefield,
                to: Zone::Graveyard,
                ..
            },
        ) => state
            .object(*object)
            .is_some_and(|o| eval::matches(filter, state, o, you, source)),
        (Trigger::SpellCast(filter), GameEvent::SpellCast { object, .. }) => state
            .object(*object)
            .is_some_and(|o| eval::matches(filter, state, o, you, source)),
        (Trigger::Draws(rel), GameEvent::CardsDrawn { player, .. }) => match rel {
            PlayerRel::You => *player == you,
            PlayerRel::Opponent => *player != you,
            _ => true,
        },
        (Trigger::StepBegin { step, whose }, GameEvent::StepChanged { .. }) => {
            let step_matches = matches!(
                (step, event),
                (
                    StepKind::Upkeep,
                    GameEvent::StepChanged {
                        step: crate::turn::Step::Upkeep,
                        ..
                    }
                ) | (
                    StepKind::Draw,
                    GameEvent::StepChanged {
                        step: crate::turn::Step::Draw,
                        ..
                    }
                ) | (
                    StepKind::End,
                    GameEvent::StepChanged {
                        step: crate::turn::Step::End,
                        ..
                    }
                ) | (
                    StepKind::CombatBegin,
                    GameEvent::StepChanged {
                        step: crate::turn::Step::CombatBegin,
                        ..
                    }
                )
            );
            if !step_matches {
                return false;
            }
            match whose {
                PlayerRel::You => state.turn.active == you,
                PlayerRel::Opponent => state.turn.active != you,
                _ => true,
            }
        }
        _ => false,
    }
}
