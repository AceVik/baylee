//! Commit all results only after every replacement decision has finished.

use super::{CounterKind, DamageTarget, DamageWork, GameState, KeywordSet, Part};
use crate::event::{Cause, GameEvent};
use baylee_core::types::TypeSet;

impl DamageWork {
    pub(super) fn commit(&mut self, state: &mut GameState) {
        // Sources and recipients are read before any result changes the
        // battlefield. The assignments themselves were already fixed before
        // prevention could remove a counter and change someone's power.
        state.refresh_characteristics();
        for part in &mut self.parts {
            if let Some(source) = state.damage_source(part.view.source, part.source_version) {
                part.keywords = source.characteristics().keywords;
                part.controller = source.controller;
            }
            if !part.recipient_exists(state) {
                part.view.amount = 0;
            }
        }
        for part in &self.parts {
            if part.view.amount == 0 {
                continue;
            }
            match part.view.recipient {
                DamageTarget::Player(player) => {
                    state.damage_player(
                        part.view.source,
                        player,
                        part.view.amount,
                        part.view.is_combat,
                        Cause::Effect,
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
            let old = state
                .object(target)
                .map_or(0, |o| o.counters.get(CounterKind::Loyalty));
            let new = old.saturating_sub(u16::try_from(n).unwrap_or(u16::MAX));
            if let Some(obj) = state.object_mut(target) {
                obj.counters.set(CounterKind::Loyalty, new);
            }
            state.journal.record(GameEvent::CounterChanged {
                object: target,
                kind: CounterKind::Loyalty,
                old,
                new,
            });
        } else if let Some(obj) = state.object_mut(target) {
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
