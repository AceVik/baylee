use super::{
    AbilityDef, CardLookup, Cause, CombatDeclared, EndReason, Engine, GameEvent, GameResult,
    ObjectId, ObjectKind, Pending, Phase, PlanKind, PlayerId, Resolution, SmallVec, Status, Step,
    ZoneLocation, ZonePosition, combat, eval, resolve, sba, trigger,
};

impl<L: CardLookup> Engine<L> {
    pub(crate) fn run_until_choice(&mut self) {
        if self.awaiting_answer {
            return;
        }
        loop {
            // 1. Game over?
            if let Some(result) = self.game_result() {
                self.pending = Pending::GameOver(result);
                self.awaiting_answer = true;
                self.state.journal.record(GameEvent::GameWon {
                    player: result.winner,
                });
                return;
            }
            // 2. State-based actions (fixpoint).
            let outcome = sba::run(&mut self.state);
            if let Some((player, options)) = outcome.legend_choice {
                self.pending = Pending::LegendChoice { player, options };
                self.awaiting_answer = true;
                return;
            }
            if outcome.changed {
                continue;
            }
            // 3. Triggers from new events.
            self.collect_triggers();
            if self.awaiting_answer {
                return; // a trigger's target choice is pending
            }
            // 4. Resolve the top of the stack after all passed.
            if self.resolve_next {
                self.resolve_next = false;
                self.passes = 0;
                self.priority_holder = None;
                self.resolve_stack_top();
                if self.awaiting_answer {
                    return; // resolution suspended on a choice
                }
                continue;
            }
            // 5. Progress the step machine.
            if self.progress_step() {
                return; // a pending choice was set
            }
        }
    }

    /// One step-machine transition. Returns `true` when a pending choice
    /// was produced (loop must stop).
    pub(crate) fn progress_step(&mut self) -> bool {
        match self.state.turn.step {
            Step::Untap => {
                self.untap_step();
                false
            }
            Step::Cleanup => self.cleanup_step(),
            Step::DeclareAttackers if self.combat_declared != CombatDeclared::Attackers => {
                self.pending = Pending::ChooseAttackers {
                    player: self.state.turn.active,
                };
                self.awaiting_answer = true;
                true
            }
            Step::DeclareBlockers if self.combat_declared != CombatDeclared::Blockers => {
                let active = self.state.turn.active;
                let defending = self.next_alive_after(active);
                self.pending = Pending::ChooseBlockers {
                    player: defending,
                    attacker: active,
                };
                self.awaiting_answer = true;
                true
            }
            _ => self.priority_round(),
        }
    }

    /// Runs the APNAP priority round for steps that grant priority.
    /// Returns `true` when a pending choice was produced.
    pub(crate) fn priority_round(&mut self) -> bool {
        if self.priority_holder.is_none() && self.passes == 0 {
            // Open a new round with the active player (CR 117.3a).
            let active = self.state.turn.active;
            self.priority_holder = Some(active);
            self.pending = Pending::Priority {
                player: active,
                legal: Box::new(self.compute_legal(active)),
            };
            self.awaiting_answer = true;
            return true;
        }
        let alive = self.alive_players();
        if self.passes >= alive.len() as u8 {
            // Round complete: resolve or advance.
            self.passes = 0;
            self.priority_holder = None;
            if self.state.zones.stack_is_empty() {
                self.advance_step();
            } else {
                self.resolve_next = true;
            }
            false
        } else {
            let current = self.priority_holder.expect("round started");
            let next = self.next_alive_after(current);
            self.priority_holder = Some(next);
            self.pending = Pending::Priority {
                player: next,
                legal: Box::new(self.compute_legal(next)),
            };
            self.awaiting_answer = true;
            true
        }
    }

    pub(crate) fn alive_players(&self) -> Vec<PlayerId> {
        self.state
            .players
            .iter()
            .filter(|p| !p.has_lost)
            .map(|p| p.id)
            .collect()
    }

    pub(crate) fn next_alive_after(&self, player: PlayerId) -> PlayerId {
        let n = self.state.players.len() as u8;
        let start = player.get();
        for offset in 1..=n {
            let candidate = PlayerId::new((start + offset) % n);
            if !self.state.players[candidate.get() as usize].has_lost {
                return candidate;
            }
        }
        player
    }

    pub(crate) fn game_result(&self) -> Option<GameResult> {
        let alive = self.alive_players();
        match alive.len() {
            0 => Some(GameResult {
                winner: None,
                reason: EndReason::Draw,
            }),
            1 => Some(GameResult {
                winner: Some(alive[0]),
                reason: EndReason::LastPlayerStanding,
            }),
            _ => None,
        }
    }

    pub(crate) fn collect_triggers(&mut self) {
        if self.trigger_queue.is_empty() {
            let found = trigger::collect(&self.state, &self.lookup, self.trigger_scan_seq);
            self.trigger_scan_seq = self.state.journal.last_seq();
            self.trigger_queue = found.into_iter().collect();
        }
        while let Some(t) = self.trigger_queue.front().copied() {
            let target_spec = self
                .state
                .object(t.source)
                .and_then(|o| o.card)
                .and_then(|c| self.lookup.card(c.index))
                .and_then(|def| def.abilities.get(t.ability_index as usize))
                .and_then(|a| match a {
                    AbilityDef::Triggered { target, .. } => *target,
                    _ => None,
                });
            if let Some(spec) = target_spec {
                let options = eval::target_options(&spec, &self.state, t.controller, t.source);
                if options.is_empty() {
                    // No legal target: the trigger is removed from the stack
                    // entirely (CR 603.3d).
                    self.trigger_queue.pop_front();
                    continue;
                }
                self.pending_plan = Some(PlanKind::Trigger {
                    source: t.source,
                    ability_index: t.ability_index,
                });
                self.pending = Pending::ChooseTargets {
                    player: t.controller,
                    options,
                    count: 1,
                };
                self.awaiting_answer = true;
                return;
            }
            self.trigger_queue.pop_front();
            let _ = self.push_ability_to_stack(
                t.controller,
                t.source,
                t.ability_index,
                SmallVec::new(),
            );
        }
    }

    pub(crate) fn resolve_stack_top(&mut self) {
        let Some(&top) = self.state.zones.list(ZoneLocation::Stack).last() else {
            return;
        };
        self.state
            .journal
            .record(GameEvent::StackObjectResolved { object: top });
        let kind = self.state.object(top).map(|o| o.kind);
        if kind == Some(ObjectKind::AbilityOnStack) {
            let obj = self.state.object(top).expect("stack object exists");
            let loc = obj.ability.expect("ability object has a location");
            let def = self.lookup.card(loc.card).expect("ability card exists");
            let effects = match def.abilities.get(loc.index as usize) {
                Some(
                    AbilityDef::Activated { effects, .. } | AbilityDef::Triggered { effects, .. },
                ) => *effects,
                _ => panic!("ability object references non-resolvable ability"),
            };
            let mut res = Resolution {
                source: loc.source,
                on_stack: top,
                controller: obj.controller,
                effects: resolve::flatten(effects),
                pc: 0,
                targets: obj.targets.clone(),
                x: None,
                awaiting: None,
            };
            match resolve::run(&mut self.state, &mut res) {
                resolve::Flow::Complete => self.finish_resolution(&res),
                resolve::Flow::Wait(pending) => {
                    self.resolution = Some(res);
                    self.pending = pending;
                    self.awaiting_answer = true;
                }
            }
            return;
        }
        // A spell resolves.
        let spell_fx = self
            .state
            .object(top)
            .and_then(|o| o.card)
            .and_then(|c| self.lookup.card(c.index))
            .and_then(|def| {
                def.abilities.iter().find_map(|a| match a {
                    AbilityDef::Spell { effects, .. } if !effects.is_empty() => Some(*effects),
                    _ => None,
                })
            });
        if let Some(fx) = spell_fx {
            let obj = self.state.object(top).expect("stack object exists");
            let mut res = Resolution {
                source: top,
                on_stack: top,
                controller: obj.controller,
                effects: resolve::flatten(fx),
                pc: 0,
                targets: obj.targets.clone(),
                x: None,
                awaiting: None,
            };
            match resolve::run(&mut self.state, &mut res) {
                resolve::Flow::Complete => self.finish_resolution(&res),
                resolve::Flow::Wait(pending) => {
                    self.resolution = Some(res);
                    self.pending = pending;
                    self.awaiting_answer = true;
                }
            }
        } else {
            self.finalize_spell(top);
        }
    }

    pub(crate) fn finish_resolution(&mut self, res: &Resolution) {
        if self
            .state
            .object(res.on_stack)
            .is_some_and(|o| o.kind == ObjectKind::AbilityOnStack)
        {
            // Abilities on the stack simply cease to exist (CR 608.2k).
            self.state.zones.remove(res.on_stack, ZoneLocation::Stack);
            let _ = self.state.arena.remove(res.on_stack);
        } else {
            self.finalize_spell(res.on_stack);
        }
    }

    pub(crate) fn finalize_spell(&mut self, spell: ObjectId) {
        let (is_permanent, owner) = {
            let Some(obj) = self.state.object(spell) else {
                return;
            };
            (obj.characteristics().types.is_permanent(), obj.owner)
        };
        if is_permanent {
            if let Some(obj) = self.state.object_mut(spell) {
                obj.kind = ObjectKind::Permanent;
                obj.status.remove(Status::TAPPED);
            }
            let _ = self.state.move_object(
                spell,
                ZoneLocation::Battlefield,
                ZonePosition::Top,
                Cause::Spell,
            );
        } else {
            if let Some(obj) = self.state.object_mut(spell) {
                obj.kind = ObjectKind::Card;
            }
            let _ = self.state.move_object(
                spell,
                ZoneLocation::Graveyard(owner),
                ZonePosition::Top,
                Cause::Spell,
            );
        }
    }

    // ------------------------------------------------------------ combat

    pub(crate) fn begin_turn(&mut self, first_turn: bool) {
        if !first_turn {
            let next = self.next_alive_after(self.state.turn.active);
            self.state.turn.active = next;
            self.state.turn.number += 1;
        }
        self.state.turn_start_timestamp = self.state.timestamp;
        let active = self.state.turn.active;
        self.state.players[active.get() as usize].lands_played_this_turn = 0;
        self.state.turn.phase = Phase::Beginning;
        self.state.turn.step = Step::Untap;
        self.combat_declared = CombatDeclared::None;
        self.state.journal.record(GameEvent::TurnStarted {
            number: self.state.turn.number,
            active,
        });
    }

    pub(crate) fn advance_step(&mut self) {
        let (phase, step) = (self.state.turn.phase, self.state.turn.step);
        let (next_phase, next_step) = match (phase, step) {
            (_, Step::Untap) => (Phase::Beginning, Step::Upkeep),
            (_, Step::Upkeep) => (Phase::Beginning, Step::Draw),
            (_, Step::Draw) => {
                // Turn-based action: draw (first player skips on turn 1 in
                // two-player games, CR 103.8).
                let skip = self.state.turn.number == 1
                    && self.state.players.len() == 2
                    && self.state.turn.active.get() == 0;
                if !skip {
                    let active = self.state.turn.active;
                    self.state.draw_cards(active, 1);
                }
                (Phase::FirstMain, Step::Main)
            }
            (Phase::FirstMain, Step::Main) => (Phase::Combat, Step::CombatBegin),
            (Phase::SecondMain, Step::Main) => (Phase::Ending, Step::End),
            (_, Step::CombatBegin) => (Phase::Combat, Step::DeclareAttackers),
            (_, Step::DeclareAttackers) => (Phase::Combat, Step::DeclareBlockers),
            (_, Step::DeclareBlockers) => {
                // Deal combat damage on entering the damage step(s).
                if self.any_first_or_double_striker() {
                    combat::deal_combat_damage(&mut self.state, true);
                    (Phase::Combat, Step::CombatDamageFirst)
                } else {
                    combat::deal_combat_damage(&mut self.state, false);
                    (Phase::Combat, Step::CombatDamage)
                }
            }
            (_, Step::CombatDamageFirst) => {
                combat::deal_combat_damage(&mut self.state, false);
                (Phase::Combat, Step::CombatDamage)
            }
            (_, Step::CombatDamage) => (Phase::Combat, Step::CombatEnd),
            (_, Step::CombatEnd) => {
                self.state.combat = crate::combat::CombatState::default();
                self.combat_declared = CombatDeclared::None;
                (Phase::SecondMain, Step::Main)
            }
            (_, Step::End) => (Phase::Ending, Step::Cleanup),
            (_, Step::Cleanup) => (Phase::Beginning, Step::Untap),
            _ => unreachable!("invalid phase/step combination"),
        };
        self.state.turn.phase = next_phase;
        self.state.turn.step = next_step;
        self.state.journal.record(GameEvent::StepChanged {
            phase: next_phase,
            step: next_step,
        });
    }

    pub(crate) fn any_first_or_double_striker(&self) -> bool {
        use baylee_cards_dsl::KeywordSet as K;
        self.state
            .combat
            .attackers
            .iter()
            .map(|a| a.creature)
            .chain(self.state.combat.blockers.iter().map(|b| b.blocker))
            .any(|id| {
                self.state.object(id).is_some_and(|o| {
                    let kw = o.characteristics().keywords;
                    kw.contains(K::FIRST_STRIKE) || kw.contains(K::DOUBLE_STRIKE)
                })
            })
    }

    pub(crate) fn untap_step(&mut self) {
        let active = self.state.turn.active;
        let battlefield = self.state.zones.list(ZoneLocation::Battlefield).clone();
        for id in battlefield {
            let tapped = self
                .state
                .object(id)
                .is_some_and(|o| o.controller == active && o.status.contains(Status::TAPPED));
            if tapped {
                if let Some(obj) = self.state.object_mut(id) {
                    obj.status.remove(Status::TAPPED);
                }
                self.state.journal.record(GameEvent::ObjectUntapped {
                    object: id,
                    cause: Cause::TurnBased,
                });
            }
        }
        self.advance_step();
    }

    pub(crate) fn cleanup_step(&mut self) -> bool {
        // Clear damage (CR 514.2) and check hand size.
        for obj in self.state.arena.iter_mut_all() {
            obj.damage = 0;
        }
        let active = self.state.turn.active;
        let max_hand = 7i32 + i32::from(self.state.players[active.get() as usize].hand_modifier);
        let hand_size = self.state.zones.list(ZoneLocation::Hand(active)).len() as i32;
        if hand_size > max_hand {
            self.pending = Pending::DiscardChoice {
                player: active,
                count: (hand_size - max_hand) as u8,
            };
            self.awaiting_answer = true;
            return true;
        }
        self.end_cleanup();
        false
    }

    pub(crate) fn end_cleanup(&mut self) {
        self.state.combat = crate::combat::CombatState::default();
        self.combat_declared = CombatDeclared::None;
        self.begin_turn(false);
    }
}
