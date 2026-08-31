use super::{
    AbilityDef, AbilityLoc, CardLookup, Cause, CombatDeclared, EndReason, Engine, GameEvent,
    GameObject, GameResult, LossReason, NameRef, ObjectId, ObjectKind, Pending, Phase, PlanKind,
    PlayerId, Resolution, SmallVec, Status, Step, Zone, ZoneLocation, ZonePosition, combat, eval,
    mana_pay, resolve, sba, trigger,
};
use crate::choice::{CastModeDesc, CastModeKind, YesNoPrompt};

impl<L: CardLookup> Engine<L> {
    pub(crate) fn run_until_choice(&mut self) {
        if self.awaiting_answer {
            return;
        }
        loop {
            // 0. Continuous effects: sync statics with the battlefield and
            //    refresh characteristic caches (generation compare).
            self.sync_static_effects();
            self.state.refresh_characteristics();
            // 0b. As-it-enters modifiers (taplands, shockland choices).
            self.apply_enter_modifiers();
            if self.awaiting_answer {
                return;
            }
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
            // 3b. Delayed actions queued by upkeep processing.
            if self.process_delayed() {
                return; // a delayed action produced a pending choice
            }
            // 3c. Miracle offers for first-of-turn draws (CR 702.94).
            if self.offer_miracle() {
                return;
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

    /// Offers the next pending miracle cast (first-of-turn draws with
    /// miracle still in hand). Returns `true` when a choice was produced.
    pub(crate) fn offer_miracle(&mut self) -> bool {
        while let Some((player, card)) = self.state.pending_miracle.pop_front() {
            let Some(obj) = self.state.object(card) else {
                continue;
            };
            if obj.zone != crate::zone::Zone::Hand || obj.zone_owner != Some(player) {
                continue;
            }
            let Some(def) = obj.card.and_then(|c| self.lookup.card(c.index)) else {
                continue;
            };
            if def.faces[obj.face_index as usize].miracle.is_none() {
                continue;
            }
            self.pending_plan = Some(PlanKind::Miracle { card });
            self.pending = Pending::YesNo {
                player,
                prompt: crate::choice::YesNoPrompt::Miracle { card },
            };
            self.awaiting_answer = true;
            return true;
        }
        false
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

    /// Applies as-it-enters-the-battlefield modifiers to permanents that
    /// entered since the last scan (CR 614.1c/d; taplands, shocklands).
    /// Returns whether anything changed (for legal-list recomputation).
    #[allow(clippy::too_many_lines)] // the entry-modifier table is naturally flat
    pub(crate) fn apply_enter_modifiers(&mut self) -> bool {
        use baylee_cards_dsl::EnterModifier;
        let events: Vec<(ObjectId, PlayerId)> = self
            .state
            .journal
            .entries()
            .get(self.entry_scan_seq as usize..)
            .unwrap_or_default()
            .iter()
            .filter_map(|e| match &e.event {
                GameEvent::ZoneChanged {
                    object,
                    to: Zone::Battlefield,
                    ..
                } => Some(*object),
                _ => None,
            })
            .filter_map(|id| self.state.object(id).map(|o| (id, o.controller)))
            .collect();
        self.entry_scan_seq = self.state.journal.last_seq();
        let mut changed = false;
        for (id, controller) in events {
            // Echo (CR 702.30): register the pay-or-sacrifice choice at
            // the controller's next upkeep.
            if let Some(cost) = self.state.object(id).and_then(|o| {
                let face = o.face_index as usize;
                o.card.and_then(|c| {
                    self.lookup.card(c.index).and_then(|def| {
                        def.abilities_for_face(face).iter().find_map(|a| match a {
                            baylee_cards_dsl::AbilityDef::Echo { cost } => Some(*cost),
                            _ => None,
                        })
                    })
                })
            }) {
                self.state.delayed.push(crate::state::DelayedTrigger {
                    controller,
                    when: crate::state::DelayedWhen::NextUpkeep,
                    action: crate::state::DelayedAction::PayCostOrSacrifice { cost, card: id },
                });
            }
            // Sagas enter with a lore counter, triggering chapter I
            // (CR 714.2a/b).
            let chapter_one = self.state.object(id).and_then(|o| {
                let face = o.face_index as usize;
                o.card.and_then(|c| {
                    self.lookup.card(c.index).and_then(|def| {
                        def.abilities_for_face(face)
                            .iter()
                            .enumerate()
                            .find_map(|(i, a)| match a {
                                baylee_cards_dsl::AbilityDef::SagaChapter {
                                    chapter: 1, ..
                                } => Some(i as u32),
                                _ => None,
                            })
                    })
                })
            });
            if let Some(ability_index) = chapter_one {
                let ts = self.state.next_timestamp();
                if let Some(obj) = self.state.object_mut(id) {
                    obj.counters.add(baylee_cards_dsl::CounterKind::Lore, 1);
                    obj.timestamp = ts;
                }
                self.trigger_queue
                    .push_back(crate::trigger::PendingTrigger {
                        source: id,
                        ability_index,
                        controller,
                        timestamp: ts,
                        event_object: None,
                        synthetic_effects: None,
                        once_per_turn: false,
                        synthetic_target: None,
                    });
                changed = true;
            }
            // Planeswalkers enter with their printed loyalty counters
            // (CR 306.5b).
            if let Some(loyalty) = self
                .state
                .object(id)
                .and_then(|o| o.card)
                .and_then(|c| self.lookup.card(c.index))
                .and_then(|def| {
                    let face = &def.faces[0];
                    if face
                        .types
                        .contains(baylee_core::types::TypeSet::PLANESWALKER)
                    {
                        face.loyalty
                    } else {
                        None
                    }
                })
            {
                // Counter-placement replacements (Doubling Season doubles
                // loyalty counters on ETB too, CR 614.16).
                let mut amount = loyalty;
                {
                    let obj = self.state.object(id).expect("walker exists");
                    for entry in &self.state.replacement_rules {
                        if let baylee_cards_dsl::ReplacementRule::DoubleCounterPlacement {
                            object_filter,
                        } = entry.rule
                            && crate::eval::matches(
                                object_filter,
                                &self.state,
                                obj,
                                entry.controller,
                                entry.source,
                            )
                        {
                            amount = amount.saturating_mul(2);
                        }
                    }
                }
                let obj = self.state.object_mut(id).expect("walker exists");
                let old = obj.counters.get(baylee_cards_dsl::CounterKind::Loyalty);
                let new = obj
                    .counters
                    .add(baylee_cards_dsl::CounterKind::Loyalty, amount);
                self.state.journal.record(GameEvent::CounterChanged {
                    object: id,
                    kind: baylee_cards_dsl::CounterKind::Loyalty,
                    old,
                    new,
                });
                changed = true;
            }
            // Clone-on-enter: offer the copy choice before anything else
            // for this permanent (CR 614.4).
            if self.check_copy_on_enter(id) {
                return true;
            }
            let Some(card) = self.state.object(id).and_then(|o| o.card) else {
                continue;
            };
            let Some(def) = self.lookup.card(card.index) else {
                continue;
            };
            for modifier in def.faces[0].enter_modifiers {
                match modifier {
                    EnterModifier::Tapped => {
                        if let Some(obj) = self.state.object_mut(id) {
                            obj.status.insert(Status::TAPPED);
                            changed = true;
                        }
                    }
                    EnterModifier::TappedUnless(filter) => {
                        let controlled = self
                            .state
                            .zones
                            .list(ZoneLocation::Battlefield)
                            .iter()
                            .any(|other| {
                                *other != id
                                    && self.state.object(*other).is_some_and(|o| {
                                        eval::matches(filter, &self.state, o, controller, id)
                                    })
                            });
                        if !controlled && let Some(obj) = self.state.object_mut(id) {
                            obj.status.insert(Status::TAPPED);
                            changed = true;
                        }
                    }
                    EnterModifier::Prepared => {
                        if let Some(obj) = self.state.object_mut(id)
                            && !obj.riders.contains(&crate::object::Rider::Prepared)
                        {
                            obj.riders.push(crate::object::Rider::Prepared);
                        }
                    }
                    EnterModifier::ChooseSubtype => {
                        self.pending_plan = Some(PlanKind::ChooseSubtype { object: id });
                        self.pending = Pending::ChooseSubtype {
                            player: controller,
                            options: (0..=349).map(baylee_core::ids::SubtypeId::new).collect(),
                        };
                        self.awaiting_answer = true;
                        return true; // one choice at a time
                    }
                    EnterModifier::TappedOrPayLife(amount) => {
                        let amount = *amount;
                        // Unpayable → tapped without a choice.
                        if self.state.players[controller.get() as usize].life <= i32::from(amount) {
                            if let Some(obj) = self.state.object_mut(id) {
                                obj.status.insert(Status::TAPPED);
                                changed = true;
                            }
                            continue;
                        }
                        self.pending_plan = Some(PlanKind::EntryTap { object: id, amount });
                        self.pending = Pending::YesNo {
                            player: controller,
                            prompt: YesNoPrompt::PayLifeOrEnterTapped { amount },
                        };
                        self.awaiting_answer = true;
                        return true; // one choice at a time
                    }
                }
            }
        }
        changed
    }

    /// Checks a newly entered permanent for a clone-on-enter clause and
    /// presents the copy choice when valid targets exist. Returns `true`
    /// when a pending choice was produced.
    pub(crate) fn check_copy_on_enter(&mut self, id: ObjectId) -> bool {
        let (spec, controller) = {
            let Some(obj) = self.state.object(id) else {
                return false;
            };
            let Some(card) = obj.card else { return false };
            let Some(def) = self.lookup.card(card.index) else {
                return false;
            };
            let Some(spec) = def
                .abilities_for_face(obj.face_index as usize)
                .iter()
                .find_map(|a| match a {
                    AbilityDef::CopyOnEnter { target, .. }
                    | AbilityDef::CopyOnEnterUntilEot { target, .. } => Some(*target),
                    _ => None,
                })
            else {
                return false;
            };
            (spec, obj.controller)
        };
        let options = eval::target_options(&spec, &self.state, controller, id);
        if options.is_empty() {
            return false; // optional: simply doesn't copy
        }
        self.pending_plan = Some(PlanKind::CopyOnEnter { object: id });
        self.pending = Pending::ChooseTargets {
            player: controller,
            options,
            min: 0,
            max: 1,
        };
        self.awaiting_answer = true;
        true
    }

    /// Applies the clone-on-enter choice: the permanent's copiable base is
    /// replaced by the target's base, with the card's modifications. For
    /// `CopyOnEnterUntilEot` (Cursed Mirror), the copy is a layer-1
    /// continuous effect with `UntilEndOfTurn` duration instead.
    #[allow(clippy::too_many_lines)]
    pub(crate) fn apply_copy_choice(&mut self, id: ObjectId, target: ObjectId) {
        let (mods, until_eot): (Vec<baylee_cards_dsl::CopyMod>, bool) = {
            let Some(obj) = self.state.object(id) else {
                return;
            };
            let Some(card) = obj.card else { return };
            let Some(def) = self.lookup.card(card.index) else {
                return;
            };
            def.abilities_for_face(obj.face_index as usize)
                .iter()
                .find_map(|a| match a {
                    AbilityDef::CopyOnEnter { mods, .. } => Some((mods.to_vec(), false)),
                    AbilityDef::CopyOnEnterUntilEot { mods, .. } => Some((mods.to_vec(), true)),
                    _ => None,
                })
                .unwrap_or_default()
        };
        if until_eot {
            // Temporary copy: layer-1 effect + mods as their own effects.
            let controller = self
                .state
                .object(id)
                .map_or(PlayerId::new(0), |o| o.controller);
            let ts = self.state.next_timestamp();
            self.state
                .effects
                .register(crate::effects::ContinuousEffect {
                    id: baylee_core::ids::EffectId::new(0),
                    source: Some(id),
                    controller,
                    layer: baylee_cards_dsl::Layer::Copy,
                    timestamp: ts,
                    duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
                    filter: crate::effects::EffectFilter::ObjectIs(id),
                    modifier: baylee_cards_dsl::Modifier::BecomeCopyOf(target),
                });
            for m in mods {
                let (layer, modifier) = match m {
                    baylee_cards_dsl::CopyMod::AddKeyword(k) => (
                        baylee_cards_dsl::Layer::Ability,
                        baylee_cards_dsl::Modifier::AddKeyword(k),
                    ),
                    baylee_cards_dsl::CopyMod::AddType(t) => (
                        baylee_cards_dsl::Layer::Type,
                        baylee_cards_dsl::Modifier::AddType(t),
                    ),
                    baylee_cards_dsl::CopyMod::RemoveType(t) => (
                        baylee_cards_dsl::Layer::Type,
                        baylee_cards_dsl::Modifier::RemoveType(t),
                    ),
                    baylee_cards_dsl::CopyMod::AddSubtype(s) => (
                        baylee_cards_dsl::Layer::Type,
                        baylee_cards_dsl::Modifier::AddSubtype(s),
                    ),
                    baylee_cards_dsl::CopyMod::AddCounter(kind, n) => {
                        if let Some(obj) = self.state.object_mut(id) {
                            obj.counters.add(kind, n);
                        }
                        continue;
                    }
                    baylee_cards_dsl::CopyMod::RemoveSupertype(_) => continue, // no modifier form
                };
                let ts = self.state.next_timestamp();
                self.state
                    .effects
                    .register(crate::effects::ContinuousEffect {
                        id: baylee_core::ids::EffectId::new(0),
                        source: Some(id),
                        controller,
                        layer,
                        timestamp: ts,
                        duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
                        filter: crate::effects::EffectFilter::ObjectIs(id),
                        modifier,
                    });
            }
            return;
        }
        let Some(target_base) = self.state.object(target).map(|o| o.base.clone()) else {
            return;
        };
        {
            let obj = self.state.object_mut(id).expect("copy target exists");
            obj.base = target_base;
        }
        for m in mods {
            let obj = self.state.object_mut(id).expect("copy target exists");
            match m {
                baylee_cards_dsl::CopyMod::AddType(t) => {
                    obj.base.types = obj.base.types.union(t);
                }
                baylee_cards_dsl::CopyMod::RemoveType(t) => {
                    obj.base.types = obj.base.types.difference(t);
                }
                baylee_cards_dsl::CopyMod::RemoveSupertype(s) => {
                    obj.base.supertypes = obj.base.supertypes.difference(s);
                }
                baylee_cards_dsl::CopyMod::AddSubtype(s) => {
                    obj.base.subtypes.insert(s);
                }
                baylee_cards_dsl::CopyMod::AddKeyword(k) => {
                    obj.base.keywords = obj.base.keywords.union(k);
                }
                baylee_cards_dsl::CopyMod::AddCounter(kind, n) => {
                    obj.counters.add(kind, n);
                }
            }
        }
        self.state.characteristics_generation = u64::MAX; // force recompute
    }

    /// Keeps the effect table in sync with the battlefield: registers
    /// static abilities of permanents, drops effects whose source left.
    pub(crate) fn sync_static_effects(&mut self) {
        use baylee_cards_dsl::Duration;
        // Drop effects whose source left the battlefield (structural
        // anthem removal).
        let gone: Vec<ObjectId> = self
            .state
            .effects
            .iter()
            .filter_map(|fx| fx.source)
            .filter(|s| {
                self.state
                    .object(*s)
                    .is_none_or(|o| o.zone != Zone::Battlefield)
            })
            .collect();
        self.state.effects.remove_where(|fx| {
            matches!(fx.duration, Duration::WhileSourceOnBattlefield)
                && fx.source.is_some_and(|s| gone.contains(&s))
        });
        // Collect statics of permanents not yet registered (then apply,
        // so the borrow of `state` ends before mutation).
        let ids: Vec<ObjectId> = self.state.zones.list(ZoneLocation::Battlefield).clone();
        let mut to_register = Vec::new();
        for id in ids {
            let Some(obj) = self.state.object(id) else {
                continue;
            };
            let Some(card) = obj.card else { continue };
            let Some(def) = self.lookup.card(card.index) else {
                continue;
            };
            for ability in def.abilities_for_face(obj.face_index as usize) {
                let AbilityDef::Static(sa) = ability else {
                    continue;
                };
                if self.state.effects.has_source_ability(id, sa.modifier) {
                    continue;
                }
                to_register.push(crate::effects::ContinuousEffect {
                    id: baylee_core::ids::EffectId::new(0),
                    source: Some(id),
                    controller: obj.controller,
                    layer: sa.layer,
                    timestamp: obj.timestamp,
                    duration: Duration::WhileSourceOnBattlefield,
                    filter: crate::effects::EffectFilter::Dsl(&sa.filter),
                    modifier: sa.modifier,
                });
            }
        }
        for fx in to_register {
            self.state.effects.register(fx);
        }
        // Sync replacement rules (drop rules of departed sources, register
        // new ones).
        let gone_rules: Vec<ObjectId> = self
            .state
            .replacement_rules
            .iter()
            .map(|r| r.source)
            .filter(|s| {
                self.state
                    .object(*s)
                    .is_none_or(|o| o.zone != Zone::Battlefield)
            })
            .collect();
        self.state
            .replacement_rules
            .retain(|r| !gone_rules.contains(&r.source));
        let mut rules_to_add = Vec::new();
        for id in self.state.zones.list(ZoneLocation::Battlefield).clone() {
            let Some(obj) = self.state.object(id) else {
                continue;
            };
            let Some(card) = obj.card else { continue };
            let Some(def) = self.lookup.card(card.index) else {
                continue;
            };
            for ability in def.abilities_for_face(obj.face_index as usize) {
                let AbilityDef::Replacement(rule) = ability else {
                    continue;
                };
                if self
                    .state
                    .replacement_rules
                    .iter()
                    .any(|r| r.source == id && r.rule == *rule)
                {
                    continue;
                }
                rules_to_add.push(crate::state::ReplacementEntry {
                    source: id,
                    controller: obj.controller,
                    rule: *rule,
                });
            }
        }
        self.state.replacement_rules.extend(rules_to_add);
    }

    fn alive_players(&self) -> Vec<PlayerId> {
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

    #[allow(clippy::too_many_lines)] // the trigger queue processor is a flat state machine
    pub(crate) fn collect_triggers(&mut self) {
        if self.trigger_queue.is_empty() {
            let found = trigger::collect(&self.state, &self.lookup, self.trigger_scan_seq);
            self.trigger_scan_seq = self.state.journal.last_seq();
            self.trigger_queue = found.into_iter().collect();
        }
        while let Some(t) = self.trigger_queue.front().cloned() {
            // "This ability triggers only once each turn." The fire is
            // recorded when the trigger goes on the stack and `ability_fires`
            // is cleared at end of turn, but nothing ever read it back, so the
            // clause did nothing at all.
            if t.once_per_turn
                && self
                    .state
                    .ability_fires
                    .contains_key(&(t.source, t.ability_index))
            {
                self.trigger_queue.pop_front();
                continue;
            }
            // Modal triggers: offer the mode choice first.
            if t.ability_index != u32::MAX
                && let Some(modes) = self
                    .state
                    .object(t.source)
                    .and_then(|o| {
                        let face = o.face_index as usize;
                        o.card
                            .and_then(|c| self.lookup.card(c.index))
                            .map(|def| def.abilities_for_face(face))
                    })
                    .and_then(|abilities| abilities.get(t.ability_index as usize))
                    .and_then(|a| match a {
                        AbilityDef::ModalTriggered { modes, .. } => Some(*modes),
                        _ => None,
                    })
            {
                let options: Vec<CastModeDesc> = modes
                    .iter()
                    .enumerate()
                    .map(|(i, _)| CastModeDesc {
                        index: i as u8,
                        kind: CastModeKind::Mode(i),
                        cost: baylee_core::mana::ManaCost::ZERO,
                    })
                    .collect();
                self.pending_plan = Some(PlanKind::ModalTrigger {
                    source: t.source,
                    ability_index: t.ability_index,
                });
                self.pending = Pending::ChooseCastMode {
                    player: t.controller,
                    options,
                };
                self.awaiting_answer = true;
                return;
            }
            let req = self
                .state
                .object(t.source)
                .and_then(|o| {
                    if let Some(emblem) = o.emblem_abilities {
                        return Some(emblem);
                    }
                    let face = o.face_index as usize;
                    o.card
                        .and_then(|c| self.lookup.card(c.index))
                        .map(|def| def.abilities_for_face(face))
                })
                .and_then(|abilities| abilities.get(t.ability_index as usize))
                .and_then(|a| match a {
                    AbilityDef::Triggered { targets, .. } => *targets,
                    AbilityDef::SagaChapter { target, .. } => {
                        target.map(baylee_cards_dsl::TargetReq::one)
                    }
                    _ => None,
                });
            if let Some(req) = req {
                if matches!(req.spec, baylee_cards_dsl::TargetSpec::EventObject) {
                    let targets: SmallVec<[ObjectId; 2]> = t.event_object.into_iter().collect();
                    self.trigger_queue.pop_front();
                    if t.once_per_turn {
                        self.state
                            .ability_fires
                            .insert((t.source, t.ability_index), 1);
                    }
                    if self
                        .state
                        .object(t.source)
                        .is_some_and(|o| o.emblem_abilities.is_some())
                    {
                        self.push_emblem_ability_to_stack(
                            t.controller,
                            t.source,
                            t.ability_index,
                            targets,
                        );
                    } else {
                        let _ = self.push_ability_to_stack(
                            t.controller,
                            t.source,
                            t.ability_index,
                            targets,
                        );
                    }
                    if let Some(event_object) = t.event_object {
                        let top = self.state.zones.list(ZoneLocation::Stack).last().copied();
                        if let Some(top) = top
                            && let Some(obj) = self.state.object_mut(top)
                        {
                            obj.event_object = Some(event_object);
                        }
                    }
                    continue;
                }
                let options = eval::target_options(&req.spec, &self.state, t.controller, t.source);
                if options.len() < req.min as usize {
                    // No legal target: the trigger is removed from the stack
                    // entirely (CR 603.3d).
                    self.trigger_queue.pop_front();
                    continue;
                }
                self.pending_plan = Some(PlanKind::Trigger {
                    source: t.source,
                    ability_index: t.ability_index,
                });
                let max = req.max.min(options.len() as u8);
                self.pending = Pending::ChooseTargets {
                    player: t.controller,
                    options,
                    min: req.min,
                    max,
                };
                self.awaiting_answer = true;
                return;
            }
            self.trigger_queue.pop_front();
            if t.once_per_turn {
                self.state
                    .ability_fires
                    .insert((t.source, t.ability_index), 1);
            }
            // Synthetic triggers with a target requirement (granted
            // triggered abilities): ask for the target first.
            if t.synthetic_effects.is_some()
                && let Some(spec) = t.synthetic_target
            {
                let options = eval::target_options(&spec, &self.state, t.controller, t.source);
                if options.is_empty() {
                    self.trigger_queue.pop_front(); // fizzles (no legal target)
                    continue;
                }
                let plan_t = t.clone();
                self.pending_plan = Some(PlanKind::SyntheticTriggerTarget { trigger: plan_t });
                self.pending = Pending::ChooseTargets {
                    player: t.controller,
                    options,
                    min: 1,
                    max: 1,
                };
                self.awaiting_answer = true;
                return;
            }
            if let Some(synthetic) = t.synthetic_effects.as_ref() {
                // Synthetic keyword trigger: effects live in the side map.
                let card = self
                    .state
                    .object(t.source)
                    .and_then(|o| o.card)
                    .map(|c| c.index);
                if let Some(card) = card {
                    let name = self
                        .state
                        .object(t.source)
                        .map_or(NameRef::new(0), |o| o.base.name);
                    // The event object doubles as the implicit target
                    // (prowess: itself; ward: the targeting spell).
                    let targets: SmallVec<[ObjectId; 2]> = t.event_object.into_iter().collect();
                    let id = self.state.arena.insert_with(|id| {
                        GameObject::new_ability_on_stack(
                            id,
                            t.controller,
                            AbilityLoc {
                                card,
                                index: u32::MAX,
                                source: t.source,
                            },
                            targets,
                            name,
                        )
                    });
                    self.synthetic_fx.insert(id, synthetic);
                    self.state
                        .zones
                        .insert(id, ZoneLocation::Stack, ZonePosition::Top);
                    self.state.journal.record(GameEvent::AbilityTriggered {
                        object: id,
                        source: t.source,
                        ability_index: u32::MAX,
                        controller: t.controller,
                    });
                }
            } else {
                if self
                    .state
                    .object(t.source)
                    .is_some_and(|o| o.emblem_abilities.is_some())
                {
                    self.push_emblem_ability_to_stack(
                        t.controller,
                        t.source,
                        t.ability_index,
                        SmallVec::new(),
                    );
                } else {
                    let _ = self.push_ability_to_stack(
                        t.controller,
                        t.source,
                        t.ability_index,
                        SmallVec::new(),
                    );
                }
                // Carry the event object onto the fresh stack object.
                if let Some(event_object) = t.event_object {
                    let top = self.state.zones.list(ZoneLocation::Stack).last().copied();
                    if let Some(top) = top
                        && let Some(obj) = self.state.object_mut(top)
                    {
                        obj.event_object = Some(event_object);
                    }
                }
            }
        }
    }

    #[allow(clippy::too_many_lines)] // resolution dispatch is a flat router; extraction would obscure it
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
            // Emblem abilities resolve from the source object (no card).
            let emblem_abilities = self
                .state
                .object(loc.source)
                .and_then(|o| o.emblem_abilities);
            let def = if emblem_abilities.is_none() {
                Some(self.lookup.card(loc.card).expect("ability card exists"))
            } else {
                None
            };
            let face = self
                .state
                .object(loc.source)
                .map_or(0, |o| o.face_index as usize);
            let effects = if let Some(abilities) = emblem_abilities {
                match abilities.get(loc.index as usize) {
                    Some(AbilityDef::Triggered { effects, .. }) => *effects,
                    _ => panic!("emblem ability index out of range"),
                }
            } else if loc.index == u32::MAX {
                // Synthetic keyword trigger (prowess, ward): effects live
                // in the side map, resolved below.
                &[][..]
            } else {
                let def = def.expect("non-emblem ability has a card");
                match def.abilities_for_face(face).get(loc.index as usize) {
                    Some(
                        AbilityDef::Activated { effects, .. }
                        | AbilityDef::Triggered { effects, .. }
                        | AbilityDef::Loyalty { effects, .. }
                        | AbilityDef::SagaChapter { effects, .. },
                    ) => *effects,
                    Some(AbilityDef::ModalTriggered { modes, .. }) => {
                        let idx = obj.mode_index.map_or(0, |i| i as usize);
                        modes
                            .get(idx)
                            .map(|m| m.effects)
                            .expect("modal trigger mode exists")
                    }
                    _ => panic!(
                        "ability object references non-resolvable ability: {} index {} face {}",
                        def.name(),
                        loc.index,
                        face
                    ),
                }
            };
            if loc.index == u32::MAX {
                // Synthetic keyword trigger (prowess & co.): effects live in
                // the side map instead of the card definition.
                let synthetic = self
                    .synthetic_fx
                    .remove(&top)
                    .expect("synthetic trigger has effects");
                let mut res = Resolution {
                    source: loc.source,
                    on_stack: top,
                    controller: obj.controller,
                    effects: resolve::flatten(synthetic),
                    pc: 0,
                    targets: obj.targets.clone(),
                    x: None,
                    chosen_player: obj.chosen_player,
                    event_object: obj.event_object,
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
            let mut res = Resolution {
                source: loc.source,
                on_stack: top,
                controller: obj.controller,
                effects: resolve::flatten(effects),
                pc: 0,
                targets: obj.targets.clone(),
                x: None,
                chosen_player: obj.chosen_player,
                event_object: obj.event_object,
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
        // A spell resolves (plain spells and modal spells alike — the
        // chosen mode is stored on the spell object).
        let spell_fx = self
            .state
            .object(top)
            .and_then(|o| {
                let face = o.face_index as usize;
                o.card
                    .and_then(|c| self.lookup.card(c.index))
                    .map(|def| def.abilities_for_face(face))
            })
            .and_then(|abilities| {
                abilities.iter().find_map(|a| match a {
                    AbilityDef::Spell { effects, .. } if !effects.is_empty() => Some(*effects),
                    _ => None,
                })
            })
            .or_else(|| {
                let mode_index = self.state.object(top)?.mode_index?;
                let face = self.state.object(top)?.face_index as usize;
                let def = self
                    .state
                    .object(top)
                    .and_then(|o| o.card)
                    .and_then(|c| self.lookup.card(c.index))?;
                def.abilities_for_face(face).iter().find_map(|a| match a {
                    AbilityDef::ModalSpell { modes } => {
                        modes.get(mode_index as usize).map(|m| m.effects)
                    }
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
                x: Some(obj.x_value),
                chosen_player: obj.chosen_player,
                event_object: None,
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

    /// Applies a face switch queued by a resolution effect (transforms).
    pub(crate) fn apply_pending_face_changes(&mut self) {
        let pending: Vec<(ObjectId, u8)> = self
            .state
            .arena
            .iter()
            .filter_map(|(id, o)| o.pending_face_change.map(|f| (id, f)))
            .collect();
        for (id, face) in pending {
            if let Some(obj) = self.state.object_mut(id) {
                obj.pending_face_change = None;
            }
            if let Some(def) = self
                .state
                .object(id)
                .and_then(|o| o.card)
                .and_then(|c| self.lookup.card(c.index))
            {
                self.state.switch_face(id, def, face as usize);
            }
        }
    }

    pub(crate) fn finish_resolution(&mut self, res: &Resolution) {
        if self
            .state
            .object(res.on_stack)
            .is_some_and(|o| o.kind == ObjectKind::AbilityOnStack)
        {
            // Sagas (CR 714.4): when a chapter ability has resolved and
            // the source's lore counters cover its final chapter,
            // sacrifice it.
            let source = res.source;
            let is_chapter = self
                .state
                .object(source)
                .and_then(|o| o.card)
                .and_then(|c| self.lookup.card(c.index))
                .is_some_and(|def| {
                    let face = self
                        .state
                        .object(source)
                        .map_or(0, |o| o.face_index as usize);
                    let max = def
                        .abilities_for_face(face)
                        .iter()
                        .filter_map(|a| match a {
                            baylee_cards_dsl::AbilityDef::SagaChapter { chapter, .. } => {
                                Some(*chapter)
                            }
                            _ => None,
                        })
                        .max()
                        .unwrap_or(0);
                    max > 0
                        && self.state.object(source).is_some_and(|o| {
                            o.counters.get(baylee_cards_dsl::CounterKind::Lore) >= u16::from(max)
                        })
                });
            // Abilities on the stack simply cease to exist (CR 608.2k).
            self.state.zones.remove(res.on_stack, ZoneLocation::Stack);
            let _ = self.state.arena.remove(res.on_stack);
            if is_chapter {
                let owner = self
                    .state
                    .object(source)
                    .map_or(res.controller, |o| o.owner);
                if let Some(obj) = self.state.object_mut(source) {
                    obj.kind = ObjectKind::Card;
                }
                let _ = self.state.move_object(
                    source,
                    ZoneLocation::Graveyard(owner),
                    ZonePosition::Top,
                    crate::event::Cause::Effect,
                );
            }
        } else {
            self.finalize_spell(res.on_stack);
        }
        self.apply_pending_face_changes();
    }

    pub(crate) fn finalize_spell(&mut self, spell: ObjectId) {
        let (is_permanent, owner) = {
            let Some(obj) = self.state.object(spell) else {
                return;
            };
            (obj.characteristics().types.is_permanent(), obj.owner)
        };
        // Adventure (CR 715): an Adventure spell resolves to exile; the
        // front face may then be cast from exile.
        let is_adventure = self
            .state
            .object(spell)
            .and_then(|o| o.card)
            .and_then(|c| self.lookup.card(c.index))
            .is_some_and(|def| {
                let face = self
                    .state
                    .object(spell)
                    .map_or(0, |o| o.face_index as usize);
                def.faces.get(face).is_some_and(|f| f.adventure)
            });
        if is_adventure {
            if let Some(obj) = self.state.object_mut(spell) {
                obj.kind = ObjectKind::Card;
                obj.riders.push(crate::object::Rider::Adventure);
            }
            let _ = self.state.move_object(
                spell,
                ZoneLocation::Exile(owner),
                ZonePosition::Top,
                Cause::Effect,
            );
            return;
        }
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
            // Rebound (CR 702.88): cast from hand → exile with a rebound
            // rider and a delayed re-cast at the next upkeep.
            let rebound = self.state.object(spell).is_some_and(|o| {
                o.cast_from_hand
                    && o.characteristics()
                        .keywords
                        .contains(baylee_cards_dsl::KeywordSet::REBOUND)
            });
            if rebound {
                if let Some(obj) = self.state.object_mut(spell) {
                    obj.kind = ObjectKind::Card;
                    obj.riders.push(crate::object::Rider::Rebound);
                }
                let _ = self.state.move_object(
                    spell,
                    ZoneLocation::Exile(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
                self.state.delayed.push(crate::state::DelayedTrigger {
                    controller: owner,
                    when: crate::state::DelayedWhen::NextUpkeep,
                    action: crate::state::DelayedAction::CastFromExileWithoutPaying { card: spell },
                });
                return;
            }
            // Flashback (CR 702.34): exile instead of the graveyard.
            let flashback = self
                .state
                .object(spell)
                .is_some_and(|o| o.riders.contains(&crate::object::Rider::Flashback));
            if flashback {
                if let Some(obj) = self.state.object_mut(spell) {
                    obj.kind = ObjectKind::Card;
                }
                let _ = self.state.move_object(
                    spell,
                    ZoneLocation::Exile(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
                return;
            }
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
            // Extra turns (CR 500.7) preempt the normal successor.
            let next = self
                .state
                .extra_turns
                .pop_front()
                .unwrap_or_else(|| self.next_alive_after(self.state.turn.active));
            self.state.turn.active = next;
            self.state.turn.number += 1;
        }
        self.state.turn_start_timestamp = self.state.timestamp;
        self.state.turn_start_seq = self.state.journal.last_seq();
        // "Until your next turn" effects end as their controller's turn
        // begins (Elspeth's flying, Teferi's sorcery-flash).
        let new_active = self.state.turn.active;
        self.state.effects.remove_where(|fx| {
            matches!(fx.duration, baylee_cards_dsl::Duration::UntilYourNextTurn)
                && fx.controller == new_active
        });
        self.state.per_turn.reset();
        self.state.ability_fires.clear();
        self.loyalty_used_this_turn.clear();
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

    /// Queues delayed actions (suspend finishes, pact payments) when the
    /// upkeep step begins.
    pub(crate) fn queue_upkeep_delayed(&mut self) {
        let active = self.state.turn.active;
        // Suspend countdown: decrement time counters, cast at zero.
        let suspended: Vec<ObjectId> = self
            .state
            .zones
            .list(ZoneLocation::Exile(active))
            .iter()
            .filter(|id| {
                self.state.object(**id).is_some_and(|o| {
                    o.riders
                        .iter()
                        .any(|r| matches!(r, crate::object::Rider::Suspend))
                })
            })
            .copied()
            .collect();
        for card in suspended {
            let remaining = self
                .state
                .object(card)
                .map_or(0, |o| o.counters.get(baylee_cards_dsl::CounterKind::Time));
            if remaining <= 1 {
                // Last counter removed: cast it without paying (CR 702.61).
                if let Some(obj) = self.state.object_mut(card) {
                    obj.counters.set(baylee_cards_dsl::CounterKind::Time, 0);
                }
                self.delayed_queue
                    .push_back(crate::state::DelayedAction::CastFromExileWithoutPaying { card });
            } else if let Some(obj) = self.state.object_mut(card) {
                obj.counters
                    .set(baylee_cards_dsl::CounterKind::Time, remaining - 1);
                self.state.journal.record(GameEvent::CounterChanged {
                    object: card,
                    kind: baylee_cards_dsl::CounterKind::Time,
                    old: remaining,
                    new: remaining - 1,
                });
            }
        }
        // Delayed triggers registered for this upkeep.
        let mut i = 0;
        while i < self.state.delayed.len() {
            let fire = matches!(
                self.state.delayed[i].when,
                crate::state::DelayedWhen::NextUpkeep
            ) && self.state.delayed[i].controller == active;
            if fire {
                let trigger = self.state.delayed.remove(i);
                self.delayed_queue.push_back(trigger.action);
            } else {
                i += 1;
            }
        }
    }

    /// Pushes a synthetic trigger (prowess, ward, granted abilities) with
    /// explicitly chosen targets onto the stack.
    pub(crate) fn push_synthetic_trigger_with_targets(
        &mut self,
        t: &crate::trigger::PendingTrigger,
        targets: SmallVec<[ObjectId; 2]>,
    ) {
        let Some(synthetic) = t.synthetic_effects else {
            return;
        };
        let Some(card) = self
            .state
            .object(t.source)
            .and_then(|o| o.card)
            .map(|c| c.index)
        else {
            return;
        };
        let name = self
            .state
            .object(t.source)
            .map_or(NameRef::new(0), |o| o.base.name);
        let id = self.state.arena.insert_with(|id| {
            GameObject::new_ability_on_stack(
                id,
                t.controller,
                AbilityLoc {
                    card,
                    index: u32::MAX,
                    source: t.source,
                },
                targets,
                name,
            )
        });
        self.synthetic_fx.insert(id, synthetic);
        self.state
            .zones
            .insert(id, ZoneLocation::Stack, ZonePosition::Top);
        self.state.journal.record(GameEvent::AbilityTriggered {
            object: id,
            source: t.source,
            ability_index: u32::MAX,
            controller: t.controller,
        });
    }

    /// Queues delayed actions that fire at the first main phase (Mana
    /// Drain's mana).
    pub(crate) fn queue_first_main_delayed(&mut self) {
        let active = self.state.turn.active;
        let mut i = 0;
        while i < self.state.delayed.len() {
            let fire = matches!(
                self.state.delayed[i].when,
                crate::state::DelayedWhen::NextFirstMain
            ) && self.state.delayed[i].controller == active;
            if fire {
                let trigger = self.state.delayed.remove(i);
                self.delayed_queue.push_back(trigger.action);
            } else {
                i += 1;
            }
        }
    }

    /// After the draw step, each saga the active player controls gets a
    /// lore counter, triggering its next chapter (CR 714.2b).
    pub(crate) fn saga_draw_step_counters(&mut self) {
        let active = self.state.turn.active;
        for id in self.state.zones.list(ZoneLocation::Battlefield).clone() {
            let Some(obj) = self.state.object(id) else {
                continue;
            };
            if obj.controller != active {
                continue;
            }
            let next_chapter = obj.counters.get(baylee_cards_dsl::CounterKind::Lore) + 1;
            let hit = obj.card.and_then(|c| {
                let face = obj.face_index as usize;
                self.lookup.card(c.index).and_then(|def| {
                    def.abilities_for_face(face)
                        .iter()
                        .enumerate()
                        .find_map(|(i, a)| match a {
                            baylee_cards_dsl::AbilityDef::SagaChapter { chapter, .. }
                                if u16::from(*chapter) == next_chapter =>
                            {
                                Some(i as u32)
                            }
                            _ => None,
                        })
                })
            });
            if let Some(ability_index) = hit {
                let ts = self.state.next_timestamp();
                if let Some(obj) = self.state.object_mut(id) {
                    obj.counters.add(baylee_cards_dsl::CounterKind::Lore, 1);
                    obj.timestamp = ts;
                }
                self.trigger_queue
                    .push_back(crate::trigger::PendingTrigger {
                        source: id,
                        ability_index,
                        controller: active,
                        timestamp: ts,
                        event_object: None,
                        synthetic_effects: None,
                        once_per_turn: false,
                        synthetic_target: None,
                    });
            }
        }
    }

    /// Queues delayed actions that fire at the beginning of the end step
    /// (Venser +2's returned permanents) — fires for ANY controller, not
    /// just the active player.
    pub(crate) fn queue_end_step_delayed(&mut self) {
        let mut i = 0;
        while i < self.state.delayed.len() {
            if matches!(
                self.state.delayed[i].when,
                crate::state::DelayedWhen::NextEndStep
            ) {
                let trigger = self.state.delayed.remove(i);
                self.delayed_queue.push_back(trigger.action);
            } else {
                i += 1;
            }
        }
    }

    /// Processes one queued delayed action; returns `true` when a pending
    /// choice was produced.
    pub(crate) fn process_delayed(&mut self) -> bool {
        let Some(action) = self.delayed_queue.pop_front() else {
            return false;
        };
        match action {
            crate::state::DelayedAction::CastFromExileWithoutPaying { card } => {
                let owner = self
                    .state
                    .object(card)
                    .map_or(self.state.turn.active, |o| o.owner);
                let _ = self.start_free_cast(owner, card);
                self.awaiting_answer
            }
            crate::state::DelayedAction::ReturnToBattlefield { card } => {
                if self
                    .state
                    .object(card)
                    .is_some_and(|o| o.zone == crate::zone::Zone::Exile)
                {
                    // End-step blink returns (Eerie Interlude, Swift
                    // Spiral): under the OWNER's control.
                    let owner = self.state.object(card).map(|o| o.owner);
                    if let Some(obj) = self.state.object_mut(card)
                        && let Some(owner) = owner
                    {
                        obj.controller = owner;
                    }
                    let _ = self.state.move_object(
                        card,
                        ZoneLocation::Battlefield,
                        ZonePosition::Top,
                        crate::event::Cause::Effect,
                    );
                }
                false
            }
            crate::state::DelayedAction::AddMana { color, amount } => {
                let active = self.state.turn.active;
                self.state.players[active.get() as usize]
                    .mana_pool
                    .add(color, amount);
                self.state.journal.record(GameEvent::ManaProduced {
                    player: active,
                    color,
                    amount,
                    source: None,
                });
                false
            }
            crate::state::DelayedAction::PayCostOrSacrifice { cost, card } => {
                let active = self.state.turn.active;
                let can_pay =
                    mana_pay::can_pay(&self.state.players[active.get() as usize].mana_pool, &cost);
                if !can_pay {
                    // Echo with an empty pool: sacrifice immediately.
                    let owner = self.state.object(card).map_or(active, |o| o.owner);
                    if let Some(obj) = self.state.object_mut(card) {
                        obj.kind = ObjectKind::Card;
                    }
                    let _ = self.state.move_object(
                        card,
                        ZoneLocation::Graveyard(owner),
                        ZonePosition::Top,
                        crate::event::Cause::Effect,
                    );
                    return false;
                }
                self.pending_plan = Some(PlanKind::DelayedPaySacrifice { cost, card });
                self.pending = Pending::YesNo {
                    player: active,
                    prompt: YesNoPrompt::Generic,
                };
                self.awaiting_answer = true;
                true
            }
            crate::state::DelayedAction::PayCostOrLose { cost } => {
                let active = self.state.turn.active;
                // If the player can't pay, they lose outright (no choice).
                let can_pay =
                    mana_pay::can_pay(&self.state.players[active.get() as usize].mana_pool, &cost);
                if !can_pay {
                    sba::eliminate_player(&mut self.state, active, LossReason::Life);
                    return false;
                }
                self.pending_plan = Some(PlanKind::DelayedPay { cost });
                self.pending = Pending::YesNo {
                    player: active,
                    prompt: YesNoPrompt::Generic,
                };
                self.awaiting_answer = true;
                true
            }
        }
    }

    pub(crate) fn advance_step(&mut self) {
        let (phase, step) = (self.state.turn.phase, self.state.turn.step);
        let (next_phase, next_step) = match (phase, step) {
            (_, Step::Untap) => {
                self.queue_upkeep_delayed();
                (Phase::Beginning, Step::Upkeep)
            }
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
            (Phase::FirstMain, Step::Main) => {
                self.queue_first_main_delayed();
                self.saga_draw_step_counters();
                (Phase::Combat, Step::CombatBegin)
            }
            (Phase::SecondMain, Step::Main) => {
                self.queue_end_step_delayed();
                (Phase::Ending, Step::End)
            }
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
                self.state.effects.remove_where(|fx| {
                    matches!(fx.duration, baylee_cards_dsl::Duration::UntilEndOfCombat)
                });
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
        // Monarch: at the beginning of the monarch's end step, draw (CR 718.4).
        if next_step == Step::End
            && let Some(monarch) = self.state.monarch
        {
            self.state.draw_cards(monarch, 1);
        }
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
        // Phasing: phased-out permanents the active player controls phase
        // back in at the untap step (CR 702.26b).
        for id in &battlefield {
            let phased = self
                .state
                .object(*id)
                .is_some_and(|o| o.controller == active && o.status.contains(Status::PHASED_OUT));
            if phased {
                if let Some(obj) = self.state.object_mut(*id) {
                    obj.status.remove(Status::PHASED_OUT);
                }
                self.state.journal.record(GameEvent::PhaseChanged {
                    object: *id,
                    phased_out: false,
                });
            }
        }
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
        // Clear damage (CR 514.2), expire "until end of turn" effects
        // (CR 514.2), and check hand size.
        for obj in self.state.arena.iter_mut_all() {
            obj.damage = 0;
        }
        self.state
            .effects
            .remove_where(|fx| matches!(fx.duration, baylee_cards_dsl::Duration::UntilEndOfTurn));
        // Temporary copies revert (Cursed Mirror).
        for obj in self.state.arena.iter_mut_all() {
            if let Some(original) = obj.original_base.take() {
                obj.base = *original;
            }
        }
        self.state.characteristics_generation = u64::MAX;
        let active = self.state.turn.active;
        // Reliquary Tower & co.: no maximum hand size for this player.
        let no_max = self.state.effects.iter().any(|fx| {
            matches!(fx.modifier, baylee_cards_dsl::Modifier::NoMaxHandSize)
                && fx.controller == active
        });
        let max_hand = if no_max {
            i32::MAX
        } else {
            7i32 + i32::from(self.state.players[active.get() as usize].hand_modifier)
        };
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
