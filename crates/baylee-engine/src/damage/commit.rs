//! Commit all results only after every replacement decision has finished.

use super::life_result::LifeLossBudgets;
use super::{CounterKind, DamageTarget, DamageWork, GameState, KeywordSet, Part};
use crate::event::Cause;
use baylee_core::types::TypeSet;

impl DamageWork {
    pub(super) fn commit(&mut self, state: &mut GameState) {
        // Sources were fixed before result replacements were chosen. Only
        // a concession can have removed a recipient while that choice waited.
        for part in &mut self.parts {
            if !part.recipient_exists(state) {
                part.view.amount = 0;
            }
        }
        let mut life_loss = LifeLossBudgets::new(self, state);
        for part in &self.parts {
            if part.view.amount == 0 {
                continue;
            }
            match part.view.recipient {
                DamageTarget::Player(player) => {
                    state.record_player_damage(part.view.source, part.source_version, player);
                    if part.keywords.contains(KeywordSet::INFECT) {
                        state.change_player_counters(
                            player,
                            CounterKind::Poison,
                            i32::try_from(part.view.amount).unwrap_or(i32::MAX),
                        );
                    } else {
                        let loss = life_loss.loss(part.view.recipient, part.view.amount);
                        if loss > 0 {
                            state.change_life(player, -loss, Cause::Effect);
                        }
                    }
                    state.record_player_damage_result(
                        part.view.source,
                        player,
                        part.view.amount,
                        part.view.is_combat,
                    );
                    if part.view.is_combat
                        && state
                            .commanders
                            .iter()
                            .flatten()
                            .any(|c| c.object == part.view.source)
                    {
                        let tally = &mut state.players[usize::from(player.get())].commander_damage;
                        let n = u16::try_from(part.view.amount).unwrap_or(u16::MAX);
                        if let Some((_, total)) =
                            tally.iter_mut().find(|(id, _)| *id == part.view.source)
                        {
                            *total = total.saturating_add(n);
                        } else {
                            tally.push((part.view.source, n));
                        }
                    }
                }
                DamageTarget::Object(id) => part.commit_object(state, id),
            }
        }
        for plan in self.result_plans.take().expect("damage results planned") {
            crate::replacement::counter_order::commit_plan(state, plan);
        }
        // Lifelink is a result of the same damage, before priority or SBAs.
        for part in &self.parts {
            if part.view.amount > 0 && part.keywords.contains(KeywordSet::LIFELINK) {
                state.change_life(
                    part.controller,
                    i32::try_from(part.view.amount).unwrap_or(i32::MAX),
                    Cause::Effect,
                );
            }
        }
        self.commit_additional(state);
    }

    pub(super) fn commit_additional(&mut self, state: &mut GameState) {
        for (player, amount) in self.life_gains.drain(..) {
            state.change_life(
                player,
                i32::try_from(amount).unwrap_or(i32::MAX),
                Cause::Effect,
            );
        }
        for (id, version, kind, n) in self.counter_removals.drain(..) {
            if state.object(id).is_some_and(|o| o.version == version) {
                crate::replacement::remove_counters(
                    state,
                    id,
                    kind,
                    u16::try_from(n).unwrap_or(u16::MAX),
                );
            }
        }
    }
}

impl Part {
    fn commit_object(&self, state: &mut GameState, target: baylee_core::ids::ObjectId) {
        let n = self.view.amount;
        if state
            .object(target)
            .is_some_and(|o| o.characteristics().types.contains(TypeSet::PLANESWALKER))
        {
            crate::replacement::remove_counters(
                state,
                target,
                CounterKind::Loyalty,
                u16::try_from(n).unwrap_or(u16::MAX),
            );
        }
        if state
            .object(target)
            .is_some_and(|o| o.characteristics().types.contains(TypeSet::CREATURE))
            && self.keywords.contains(KeywordSet::INFECT)
        {
            // Counter amounts were replaced before any damage result was
            // committed, and are placed together after this damage pass.
            if let Some(obj) = state.object_mut(target) {
                obj.deathtouched |= self.keywords.contains(KeywordSet::DEATHTOUCH);
            }
        } else if let Some(obj) = state.object_mut(target)
            && obj.characteristics().types.contains(TypeSet::CREATURE)
        {
            obj.damage = obj
                .damage
                .saturating_add(u16::try_from(n).unwrap_or(u16::MAX));
            obj.deathtouched |= self.keywords.contains(KeywordSet::DEATHTOUCH);
        }
        state.record_permanent_damage(
            self.view.source,
            self.source_version,
            target,
            n,
            self.view.is_combat,
        );
    }
}
