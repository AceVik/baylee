use super::{
    AttackerInfo, BlockerInfo, CardLookup, Cause, CombatDeclared, Engine, EngineError, GameEvent,
    ObjectId, Pending, PlanKind, PlayerAction, PlayerId, SmallVec, Status, Zone, ZoneLocation,
    ZonePosition, casting, combat, eval, resolve, sba,
};

impl<L: CardLookup> Engine<L> {
    #[allow(clippy::too_many_lines)] // flat match over the choice taxonomy — splitting would obscure, not clarify
    pub(crate) fn apply_inner(
        &mut self,
        player: PlayerId,
        action: PlayerAction,
    ) -> Result<(), EngineError> {
        match (&self.pending, action) {
            (Pending::Mulligan { player: p, .. }, PlayerAction::MulliganKeep) if *p == player => {
                let taken = self.mulligans[player.get() as usize];
                let bottom = self.mulligan_bottom_count(taken);
                if bottom > 0 {
                    self.pending = Pending::MulliganBottom {
                        player,
                        count: bottom,
                    };
                    self.awaiting_answer = true;
                } else {
                    self.advance_mulligan();
                }
                Ok(())
            }
            (
                Pending::Mulligan {
                    player: p, taken, ..
                },
                PlayerAction::MulliganTake,
            ) if *p == player => {
                // Hand goes back, reshuffle, draw 7 (CR 103.5).
                let hand: Vec<_> = self.state.zones.list(ZoneLocation::Hand(player)).clone();
                for card in hand {
                    self.state.move_object(
                        card,
                        ZoneLocation::Library(player),
                        ZonePosition::Bottom,
                        Cause::Effect,
                    )?;
                }
                self.state.shuffle_library(player);
                self.state.draw_cards(player, 7);
                let taken = *taken + 1;
                self.mulligans[player.get() as usize] = taken;
                self.pending = Pending::Mulligan {
                    player,
                    taken,
                    next_is_free: false,
                };
                self.awaiting_answer = true;
                Ok(())
            }
            (
                Pending::MulliganBottom { player: p, count },
                PlayerAction::ChooseObjects { objects },
            ) if *p == player => {
                if objects.len() != *count as usize {
                    return Err(EngineError::IllegalAction(
                        "must bottom exactly the required number of cards",
                    ));
                }
                for card in &objects {
                    if !self.in_hand(player, *card) {
                        return Err(EngineError::IllegalAction("card not in hand"));
                    }
                }
                for card in objects {
                    self.state.move_object(
                        card,
                        ZoneLocation::Library(player),
                        ZonePosition::Bottom,
                        Cause::Effect,
                    )?;
                }
                self.advance_mulligan();
                Ok(())
            }
            (Pending::Priority { player: p, .. }, PlayerAction::PassPriority) if *p == player => {
                self.passes += 1;
                Ok(())
            }
            (Pending::Priority { player: p, legal }, PlayerAction::PlayLand { card })
                if *p == player =>
            {
                if !legal.lands.contains(&card) {
                    return Err(EngineError::IllegalAction("land not playable now"));
                }
                casting::play_land(&mut self.state, player, card)?;
                self.after_action(player);
                Ok(())
            }
            (Pending::Priority { player: p, legal }, PlayerAction::CastSpell { card })
                if *p == player =>
            {
                if !legal.castable.contains(&card) {
                    return Err(EngineError::IllegalAction("spell not castable now"));
                }
                if let Some(spec) = self.spell_target_spec(card) {
                    let options = eval::target_options(&spec, &self.state, player, card);
                    if options.is_empty() {
                        return Err(EngineError::IllegalAction("no legal targets"));
                    }
                    self.pending_plan = Some(PlanKind::CastSpell { card });
                    self.pending = Pending::ChooseTargets {
                        player,
                        options,
                        count: 1,
                    };
                    self.awaiting_answer = true;
                    return Ok(());
                }
                casting::cast_spell(&mut self.state, player, card)?;
                self.after_action(player);
                Ok(())
            }
            (
                Pending::Priority { player: p, legal },
                PlayerAction::ActivateAbility {
                    source,
                    ability_index,
                },
            ) if *p == player => {
                if !legal.abilities.contains(&(source, ability_index)) {
                    return Err(EngineError::IllegalAction("ability not activatable"));
                }
                self.start_activation(player, source, ability_index, SmallVec::new())
            }
            (
                Pending::ChooseTargets {
                    player: p,
                    options,
                    count,
                },
                PlayerAction::ChooseObjects { objects },
            ) if *p == player => {
                if objects.len() != *count as usize || !objects.iter().all(|o| options.contains(o))
                {
                    return Err(EngineError::IllegalAction("invalid target selection"));
                }
                let plan = self.pending_plan.take().expect("target plan set");
                let targets: SmallVec<[ObjectId; 2]> = objects.into_iter().collect();
                match plan {
                    PlanKind::CastSpell { card } => {
                        casting::cast_spell(&mut self.state, player, card)?;
                        if let Some(obj) = self.state.object_mut(card) {
                            obj.targets = targets;
                        }
                        self.after_action(player);
                    }
                    PlanKind::ActivateAbility {
                        source,
                        ability_index,
                    } => {
                        self.start_activation(player, source, ability_index, targets)?;
                    }
                    PlanKind::Trigger {
                        source,
                        ability_index,
                    } => {
                        let controller = self.state.object(source).map_or(player, |o| o.controller);
                        self.push_ability_to_stack(controller, source, ability_index, targets)?;
                    }
                }
                Ok(())
            }
            (Pending::ChooseColor { player: p, options }, PlayerAction::ChooseColor(color))
                if *p == player =>
            {
                if !options.contains(&color) {
                    return Err(EngineError::IllegalAction("color not allowed"));
                }
                let mut res = self.resolution.take().expect("resolution suspended");
                match resolve::resume_with_color(&mut self.state, &mut res, color) {
                    resolve::Flow::Wait(pending) => {
                        self.resolution = Some(res);
                        self.pending = pending;
                        self.awaiting_answer = true;
                    }
                    resolve::Flow::Complete => {
                        self.finish_resolution(&res);
                    }
                }
                Ok(())
            }
            (Pending::YesNo { player: p, .. }, PlayerAction::YesNo(answer)) if *p == player => {
                let mut res = self.resolution.take().expect("resolution suspended");
                match resolve::resume_yes_no(&mut self.state, &mut res, answer) {
                    resolve::Flow::Wait(pending) => {
                        self.resolution = Some(res);
                        self.pending = pending;
                        self.awaiting_answer = true;
                    }
                    resolve::Flow::Complete => {
                        self.finish_resolution(&res);
                    }
                }
                Ok(())
            }
            (
                Pending::ChooseCards {
                    player: p,
                    options,
                    min,
                    max,
                    ..
                },
                PlayerAction::ChooseObjects { objects },
            ) if *p == player => {
                if objects.len() < *min as usize
                    || objects.len() > *max as usize
                    || !objects.iter().all(|o| options.contains(o))
                {
                    return Err(EngineError::IllegalAction("invalid card selection"));
                }
                let mut res = self.resolution.take().expect("resolution suspended");
                match resolve::resume(&mut self.state, &mut res, &objects) {
                    resolve::Flow::Wait(pending) => {
                        self.resolution = Some(res);
                        self.pending = pending;
                        self.awaiting_answer = true;
                    }
                    resolve::Flow::Complete => {
                        self.finish_resolution(&res);
                    }
                }
                Ok(())
            }
            (
                Pending::Priority { player: p, legal },
                PlayerAction::ActivateManaAbility { source },
            ) if *p == player => {
                if !legal.mana_abilities.contains(&source) {
                    return Err(EngineError::IllegalAction("mana ability not activatable"));
                }
                casting::activate_mana(&mut self.state, player, source)?;
                self.after_action(player);
                Ok(())
            }
            (
                Pending::ChooseAttackers { player: p },
                PlayerAction::DeclareAttackers { attackers },
            ) if *p == player => self.declare_attackers(player, attackers),
            (
                Pending::ChooseBlockers { player: p, .. },
                PlayerAction::DeclareBlockers { blockers },
            ) if *p == player => self.declare_blockers(player, blockers),
            (
                Pending::DiscardChoice { player: p, count },
                PlayerAction::ChooseObjects { objects },
            ) if *p == player => {
                if objects.len() != *count as usize {
                    return Err(EngineError::IllegalAction(
                        "must discard exactly the required number",
                    ));
                }
                for card in &objects {
                    if !self.in_hand(player, *card) {
                        return Err(EngineError::IllegalAction("card not in hand"));
                    }
                }
                for card in objects {
                    self.state.journal.record(GameEvent::Discarded {
                        object: card,
                        player,
                    });
                    self.state.move_object(
                        card,
                        ZoneLocation::Graveyard(player),
                        ZonePosition::Top,
                        Cause::TurnBased,
                    )?;
                }
                self.end_cleanup();
                Ok(())
            }
            (
                Pending::LegendChoice { player: p, options },
                PlayerAction::ChooseObjects { objects },
            ) if *p == player => {
                if objects.len() != 1 || !options.contains(&objects[0]) {
                    return Err(EngineError::IllegalAction(
                        "choose exactly one legendary permanent to keep",
                    ));
                }
                let options = options.clone();
                sba::apply_legend_choice(&mut self.state, player, objects[0], &options);
                Ok(())
            }
            _ => Err(EngineError::MismatchedAction),
        }
    }

    pub(crate) fn in_hand(&self, player: PlayerId, card: ObjectId) -> bool {
        self.state
            .object(card)
            .is_some_and(|o| o.zone == Zone::Hand && o.zone_owner == Some(player))
    }

    pub(crate) fn after_action(&mut self, player: PlayerId) {
        // After any non-pass action, priority returns to the acting player
        // (CR 117.3c) and the pass counter resets.
        self.passes = 0;
        self.priority_holder = Some(player);
        self.pending = Pending::Priority {
            player,
            legal: Box::new(self.compute_legal(player)),
        };
        self.awaiting_answer = true;
    }

    pub(crate) fn mulligan_bottom_count(&self, taken: u8) -> u8 {
        taken.saturating_sub(u8::from(self.house_rules.mulligan_free_first))
    }

    pub(crate) fn advance_mulligan(&mut self) {
        self.mulligan_player += 1;
        if self.mulligan_player >= self.state.players.len() {
            self.begin_turn(true);
        } else {
            let player = PlayerId::new(self.mulligan_player as u8);
            self.pending = Pending::Mulligan {
                player,
                taken: 0,
                next_is_free: self.house_rules.mulligan_free_first,
            };
            self.awaiting_answer = true;
        }
    }

    /// The automatic progression machine: SBAs, stack resolution, and
    /// step/turn transitions until a decision is required.
    pub(crate) fn declare_attackers(
        &mut self,
        player: PlayerId,
        attackers: Vec<(ObjectId, PlayerId)>,
    ) -> Result<(), EngineError> {
        let mut seen = Vec::with_capacity(attackers.len());
        for (creature, defending) in &attackers {
            if !combat::can_attack(&self.state, player, *creature) {
                return Err(EngineError::IllegalAction("creature cannot attack"));
            }
            if *defending == player || self.state.players[defending.get() as usize].has_lost {
                return Err(EngineError::IllegalAction("invalid defending player"));
            }
            if seen.contains(creature) {
                return Err(EngineError::IllegalAction("duplicate attacker"));
            }
            seen.push(*creature);
        }
        for (creature, defending) in attackers {
            let vigilance = self.state.object(creature).is_some_and(|o| {
                o.characteristics()
                    .keywords
                    .contains(baylee_cards_dsl::KeywordSet::VIGILANCE)
            });
            if !vigilance {
                if let Some(obj) = self.state.object_mut(creature) {
                    obj.status.insert(Status::TAPPED);
                }
                self.state.journal.record(GameEvent::ObjectTapped {
                    object: creature,
                    cause: Cause::TurnBased,
                });
            }
            self.state.combat.attackers.push(AttackerInfo {
                creature,
                defending,
            });
            self.state.journal.record(GameEvent::BecameAttacker {
                object: creature,
                defending,
            });
        }
        self.combat_declared = CombatDeclared::Attackers;
        self.passes = 0;
        self.priority_holder = None;
        Ok(())
    }

    pub(crate) fn declare_blockers(
        &mut self,
        defending: PlayerId,
        blockers: Vec<(ObjectId, ObjectId)>,
    ) -> Result<(), EngineError> {
        let mut seen = Vec::with_capacity(blockers.len());
        for (blocker, attacker) in &blockers {
            if !self
                .state
                .combat
                .attackers
                .iter()
                .any(|a| a.creature == *attacker)
            {
                return Err(EngineError::IllegalAction("no such attacker"));
            }
            if !combat::can_block(&self.state, defending, *blocker, *attacker) {
                return Err(EngineError::IllegalAction("creature cannot block"));
            }
            if seen.contains(blocker) {
                return Err(EngineError::IllegalAction("duplicate blocker"));
            }
            seen.push(*blocker);
        }
        // Menace: needs two blockers per attacker (CR 702.110).
        for attacker in &self.state.combat.attackers {
            let has_menace = self.state.object(attacker.creature).is_some_and(|o| {
                o.characteristics()
                    .keywords
                    .contains(baylee_cards_dsl::KeywordSet::MENACE)
            });
            if has_menace {
                let count = blockers
                    .iter()
                    .filter(|(_, a)| *a == attacker.creature)
                    .count();
                if count == 1 {
                    return Err(EngineError::IllegalAction("menace requires two blockers"));
                }
            }
        }
        for (blocker, attacker) in blockers {
            self.state
                .combat
                .blockers
                .push(BlockerInfo { blocker, attacker });
            self.state.journal.record(GameEvent::BecameBlocker {
                object: blocker,
                attacker,
            });
        }
        self.combat_declared = CombatDeclared::Blockers;
        self.passes = 0;
        self.priority_holder = None;
        Ok(())
    }

    // --------------------------------------------------------- turn steps
}
