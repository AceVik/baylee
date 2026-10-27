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
                        if let Some(by) = state
                            .damage_source(part.view.source, part.source_version)
                            .map(|s| s.controller)
                        {
                            state.put_player_counters_by(
                                by,
                                player,
                                CounterKind::Poison,
                                part.view.amount,
                            );
                        } else {
                            state.change_player_counters(
                                player,
                                CounterKind::Poison,
                                i32::try_from(part.view.amount).unwrap_or(i32::MAX),
                            );
                        }
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
                    if part.view.is_combat {
                        let commanders = state
                            .damage_source(part.view.source, part.source_version)
                            .map(|source| state.merged_commanders(source))
                            .unwrap_or_default();
                        let tally = &mut state.players[usize::from(player.get())].commander_damage;
                        let n = u16::try_from(part.view.amount).unwrap_or(u16::MAX);
                        for commander in commanders {
                            if let Some((_, total)) =
                                tally.iter_mut().find(|(id, _)| *id == commander)
                            {
                                *total = total.saturating_add(n);
                            } else {
                                tally.push((commander, n));
                            }
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
        for id in self.consumed_shields.drain(..) {
            if let Some(i) = state.shields.position(id) {
                state.shields.remove(i);
            }
        }
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
        let counters = u16::try_from(n).unwrap_or(u16::MAX);
        let types = state
            .object(target)
            .map_or(TypeSet::EMPTY, |object| object.characteristics().types);
        // CR 120.3h / 310.6: defense removal is a damage result, not a
        // limit on the damage dealt. Hybrids receive every applicable result.
        if types.contains(TypeSet::BATTLE) {
            crate::replacement::remove_counters(state, target, CounterKind::Defense, counters);
        }
        if types.contains(TypeSet::PLANESWALKER) {
            crate::replacement::remove_counters(state, target, CounterKind::Loyalty, counters);
        }
        if types.contains(TypeSet::CREATURE) {
            if let Some(obj) = state.object_mut(target) {
                if !self.keywords.contains(KeywordSet::INFECT) {
                    obj.damage = obj.damage.saturating_add(counters);
                }
                // Infect counter amounts were replaced before any damage result
                // committed, and are placed together after this damage pass.
                obj.deathtouched |= self.keywords.contains(KeywordSet::DEATHTOUCH);
            }
            if let Some(src_ver) = self.source_version
                && let Some(target_ver) = state.object(target).map(|o| o.version)
            {
                if state
                    .per_turn
                    .damage_exile_if_dies_sources
                    .contains(&(self.view.source, src_ver))
                    && !state.per_turn.exile_if_dies.contains(&(target, target_ver))
                {
                    state.per_turn.exile_if_dies.push((target, target_ver));
                }
                if state
                    .per_turn
                    .damage_cant_regenerate_sources
                    .contains(&(self.view.source, src_ver))
                    && !state
                        .per_turn
                        .cant_regenerate
                        .contains(&(target, target_ver))
                {
                    state.per_turn.cant_regenerate.push((target, target_ver));
                }
            }
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
