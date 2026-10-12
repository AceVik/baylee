//! Turn-bound attack restrictions follow the creature until its awaited turn begins.
#[allow(clippy::wildcard_imports)] // parent engine vocabulary
use super::*;
use crate::effects::{ContinuousEffect, EffectFilter, EffectOrigin};
use crate::state::{DelayedAction, DelayedWhen};
use baylee_cards_dsl::{Duration, Modifier};
use baylee_core::ids::EffectId;

impl<L: CardLookup> Engine<L> {
    pub(super) fn start_awaited_attack_restrictions(&mut self) {
        if !self
            .state
            .delayed
            .iter()
            .any(|delayed| delayed.when == DelayedWhen::NextAffectedTurn)
        {
            return;
        }
        // Beginning-of-turn expirations can change the controller. Bind the
        // awaited turn only after projecting those changes, not from stale arena data.
        self.state.refresh_characteristics();
        let active = self.state.turn.active;
        let arena = &self.state.arena;
        let mut due = Vec::new();
        self.state.delayed.retain(|delayed| {
            if delayed.when != DelayedWhen::NextAffectedTurn {
                return true;
            }
            let DelayedAction::RestrictAttacksForTurn { source, affected } = delayed.action else {
                return true;
            };
            let Some(object) = arena.get(affected.object).filter(|object| {
                object.zone == Zone::Battlefield && object.version == affected.version
            }) else {
                return false;
            };
            if object.controller != active {
                return true;
            }
            due.push((delayed.controller, source, affected));
            false
        });
        for (controller, source, affected) in due {
            let timestamp = self.state.next_timestamp();
            self.state.effects.register(ContinuousEffect {
                id: EffectId::new(0),
                source: Some(source.object),
                controller,
                origin: EffectOrigin::Resolution,
                layer: Modifier::CantAttack.layer(),
                timestamp,
                duration: Duration::UntilEndOfTurn,
                filter: EffectFilter::ObjectIs(affected.object, affected.version),
                modifier: Modifier::CantAttack,
            });
        }
    }
}
