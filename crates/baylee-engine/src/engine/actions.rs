use super::{
    AbilityDef, AttackerInfo, BlockerInfo, CardLookup, Cause, CombatDeclared, Engine, EngineError,
    GameEvent, LossReason, ObjectId, Pending, PlanKind, PlayerAction, PlayerId, SmallVec, Status,
    Zone, ZoneLocation, ZonePosition, cast_wizard, casting, combat, mana_pay, resolve, sba,
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
                // MDFC: which land face is played (CR 712.4a)?
                let land_faces: Vec<usize> = self
                    .state
                    .object(card)
                    .and_then(|o| o.card)
                    .and_then(|c| self.lookup.card(c.index))
                    .map_or_else(
                        || vec![0],
                        |def| {
                            def.faces
                                .iter()
                                .enumerate()
                                .filter(|(_, f)| {
                                    f.types.contains(baylee_core::types::TypeSet::LAND)
                                })
                                .map(|(i, _)| i)
                                .collect()
                        },
                    );
                if land_faces.len() > 1 {
                    // Both faces are lands (pathways): choose.
                    let options = land_faces
                        .iter()
                        .map(|&i| crate::choice::CastModeDesc {
                            index: i as u8,
                            kind: crate::choice::CastModeKind::PlayLandFace(i),
                            cost: baylee_core::mana::ManaCost::ZERO,
                        })
                        .collect();
                    self.pending_plan = Some(PlanKind::PlayLandFace { card });
                    self.pending = Pending::ChooseCastMode { player, options };
                    self.awaiting_answer = true;
                    return Ok(());
                }
                if let Some(&face) = land_faces.first()
                    && face > 0
                {
                    let def = self
                        .state
                        .object(card)
                        .and_then(|o| o.card)
                        .and_then(|c| self.lookup.card(c.index))
                        .expect("land card known");
                    self.state.switch_face(card, def, face);
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
                self.start_cast_wizard(player, card)
            }
            (Pending::ChooseCastMode { player: p, .. }, PlayerAction::ChooseMode(index))
                if *p == player =>
            {
                // MDFC land-face choice (pathways).
                if let Some(PlanKind::PlayLandFace { card }) = self.pending_plan.take() {
                    let def = self
                        .state
                        .object(card)
                        .and_then(|o| o.card)
                        .and_then(|c| self.lookup.card(c.index))
                        .expect("land card known");
                    self.state.switch_face(card, def, index);
                    casting::play_land(&mut self.state, player, card)?;
                    self.after_action(player);
                    return Ok(());
                }
                // Modal trigger mode choice.
                if let Some(PlanKind::ModalTrigger {
                    source,
                    ability_index,
                }) = self.pending_plan.take()
                {
                    self.trigger_queue.pop_front();
                    let controller = self.state.object(source).map_or(player, |o| o.controller);
                    self.push_ability_to_stack(controller, source, ability_index, SmallVec::new())?;
                    // Set the chosen mode on the fresh stack object.
                    let top = self
                        .state
                        .zones
                        .list(ZoneLocation::Stack)
                        .last()
                        .copied()
                        .expect("just pushed");
                    if let Some(obj) = self.state.object_mut(top) {
                        obj.mode_index = Some(index as u8);
                    }
                    return Ok(());
                }
                let mut wizard = self.cast_wizard.take().expect("wizard active");
                let Some(option) = wizard.options.get(index).map(|o| o.kind) else {
                    return Err(EngineError::IllegalAction("no such cast mode"));
                };
                wizard.option = Some(option);
                wizard.stage = cast_wizard::WizardStage::Targets;
                self.cast_wizard = Some(wizard);
                self.advance_cast_wizard()
            }
            (Pending::ChooseNumber { player: p, .. }, PlayerAction::ChooseNumber(n))
                if *p == player =>
            {
                let mut wizard = self.cast_wizard.take().expect("wizard active");
                wizard.x = n;
                wizard.stage = cast_wizard::WizardStage::Targets;
                self.cast_wizard = Some(wizard);
                self.advance_cast_wizard()
            }
            (Pending::ChoosePlayer { player: p, .. }, PlayerAction::ChoosePlayer(chosen))
                if *p == player =>
            {
                // Loyalty ability target player.
                if let Some(PlanKind::LoyaltyPlayer {
                    source,
                    ability_index,
                }) = self.pending_plan.take()
                {
                    self.loyalty_player_choice = Some(chosen);
                    return self.finish_loyalty_activation(
                        player,
                        source,
                        ability_index,
                        SmallVec::new(),
                    );
                }
                let mut wizard = self.cast_wizard.take().expect("wizard active");
                wizard.chosen_player = Some(chosen);
                wizard.stage = cast_wizard::WizardStage::Kicker;
                self.cast_wizard = Some(wizard);
                self.advance_cast_wizard()
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
            (Pending::Priority { player: p, legal }, PlayerAction::Suspend { card })
                if *p == player =>
            {
                if !legal.suspendable.contains(&card) {
                    return Err(EngineError::IllegalAction("card cannot be suspended"));
                }
                let counters = self
                    .state
                    .object(card)
                    .and_then(|o| o.card)
                    .and_then(|c| self.lookup.card(c.index))
                    .and_then(|def| {
                        def.abilities.iter().find_map(|a| match a {
                            AbilityDef::Suspend { counters } => Some(*counters),
                            _ => None,
                        })
                    })
                    .ok_or(EngineError::IllegalAction("not a suspend card"))?;
                let owner = self.state.object(card).map_or(player, |o| o.owner);
                {
                    let obj = self.state.object_mut(card).expect("validated");
                    obj.riders.push(crate::object::Rider::Suspend);
                    obj.counters
                        .add(baylee_cards_dsl::CounterKind::Time, u16::from(counters));
                }
                self.state.move_object(
                    card,
                    ZoneLocation::Exile(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                )?;
                self.after_action(player);
                Ok(())
            }
            (
                Pending::ChooseTargets {
                    player: p,
                    options,
                    min,
                    max,
                },
                PlayerAction::ChooseObjects { objects },
            ) if *p == player => {
                if objects.len() < *min as usize
                    || objects.len() > *max as usize
                    || !objects.iter().all(|o| options.contains(o))
                {
                    return Err(EngineError::IllegalAction("invalid target selection"));
                }
                // Wizard path: targets go to the active cast wizard.
                if self.cast_wizard.is_some() {
                    let mut wizard = self.cast_wizard.take().expect("wizard active");
                    wizard.targets = objects.into_iter().collect();
                    wizard.stage = cast_wizard::WizardStage::Kicker;
                    self.cast_wizard = Some(wizard);
                    return self.advance_cast_wizard();
                }
                let plan = self.pending_plan.take().expect("target plan set");
                let targets: SmallVec<[ObjectId; 2]> = objects.into_iter().collect();
                match plan {
                    PlanKind::ActivateAbility {
                        source,
                        ability_index,
                    } => {
                        // Loyalty abilities complete via their own finish path
                        // (no guard, no re-payment).
                        if matches!(
                            self.state
                                .object(source)
                                .and_then(|o| {
                                    let face = o.face_index as usize;
                                    o.card
                                        .and_then(|c| self.lookup.card(c.index))
                                        .map(|def| def.abilities_for_face(face))
                                })
                                .and_then(|abilities| abilities.get(ability_index as usize)),
                            Some(AbilityDef::Loyalty { .. })
                        ) {
                            self.finish_loyalty_activation(player, source, ability_index, targets)?;
                        } else {
                            self.start_activation(player, source, ability_index, targets)?;
                        }
                    }
                    PlanKind::Trigger {
                        source,
                        ability_index,
                    } => {
                        // Consume the queued trigger before stacking it.
                        self.trigger_queue.pop_front();
                        let controller = self.state.object(source).map_or(player, |o| o.controller);
                        self.push_ability_to_stack(controller, source, ability_index, targets)?;
                    }
                    PlanKind::EntryTap { .. } => {
                        unreachable!("entry-tap plans are answered via YesNo")
                    }
                    PlanKind::DelayedPay { .. } => {
                        unreachable!("delayed-pay plans are answered via YesNo")
                    }
                    PlanKind::LoyaltyPlayer { .. } => {
                        unreachable!("loyalty-player plans are answered via ChoosePlayer")
                    }
                    PlanKind::ModalTrigger { .. } => {
                        unreachable!("modal-trigger plans are answered via ChooseMode")
                    }
                    PlanKind::CopyOnEnter { object } => {
                        if let Some(&target) = targets.first() {
                            self.apply_copy_choice(object, target);
                        }
                    }
                    PlanKind::ChooseSubtype { .. } => {
                        unreachable!("subtype plans are answered via ChooseSubtype")
                    }
                    PlanKind::PlayLandFace { .. } => {
                        unreachable!("land-face plans are answered via ChooseMode")
                    }
                }
                Ok(())
            }
            (
                Pending::ChooseSubtype { player: p, options },
                PlayerAction::ChooseSubtype(subtype),
            ) if *p == player => {
                if !options.contains(&subtype) {
                    return Err(EngineError::IllegalAction("not a creature type"));
                }
                let Some(PlanKind::ChooseSubtype { object }) = self.pending_plan.take() else {
                    return Err(EngineError::IllegalAction("no subtype choice pending"));
                };
                if let Some(obj) = self.state.object_mut(object) {
                    obj.chosen_subtype = Some(subtype);
                    // "This creature is the chosen type in addition to its
                    // other types" (Roaming Throne) — creatures gain the
                    // chosen subtype in their base characteristics.
                    if obj
                        .characteristics()
                        .types
                        .contains(baylee_core::types::TypeSet::CREATURE)
                    {
                        obj.base.subtypes.insert(subtype);
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
            (Pending::OrderObjects { player: p, .. }, PlayerAction::OrderObjects { objects })
                if *p == player =>
            {
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
            (Pending::YesNo { player: p, .. }, PlayerAction::YesNo(answer)) if *p == player => {
                // Delayed pay-or-lose (Pact of Negation).
                if matches!(self.pending_plan, Some(PlanKind::DelayedPay { .. })) {
                    let Some(PlanKind::DelayedPay { cost }) = self.pending_plan.take() else {
                        unreachable!()
                    };
                    if answer {
                        debug_assert!(mana_pay::pay(
                            &mut self.state.players[player.get() as usize].mana_pool,
                            &cost,
                        ));
                    } else {
                        sba::eliminate_player(&mut self.state, player, LossReason::Life);
                    }
                    return Ok(());
                }
                // Tax choice (Rhystic Study & co.).
                if self.resolution.as_ref().is_some_and(|r| {
                    matches!(
                        r.awaiting,
                        Some(crate::resolve::AwaitingOp::PlayerMayPay { .. })
                    )
                }) {
                    let mut res = self.resolution.take().expect("resolution suspended");
                    match resolve::resume_tax_choice(&mut self.state, &mut res, answer) {
                        resolve::Flow::Wait(pending) => {
                            self.resolution = Some(res);
                            self.pending = pending;
                            self.awaiting_answer = true;
                        }
                        resolve::Flow::Complete => {
                            self.finish_resolution(&res);
                        }
                    }
                    return Ok(());
                }
                // Shockland entry choice: pay life or enter tapped.
                if matches!(self.pending_plan, Some(PlanKind::EntryTap { .. })) {
                    let Some(PlanKind::EntryTap { object, amount }) = self.pending_plan.take()
                    else {
                        unreachable!()
                    };
                    if answer {
                        let p_ref = &mut self.state.players[player.get() as usize];
                        let old = p_ref.life;
                        p_ref.life -= i32::from(amount);
                        let new = p_ref.life;
                        self.state.journal.record(GameEvent::LifeChanged {
                            player,
                            old,
                            new,
                            cause: Cause::Cost,
                        });
                    } else if let Some(obj) = self.state.object_mut(object) {
                        obj.status.insert(Status::TAPPED);
                    }
                    return Ok(());
                }
                // Wizard path: kicker yes/no.
                if self
                    .cast_wizard
                    .as_ref()
                    .is_some_and(|w| w.stage == cast_wizard::WizardStage::Kicker)
                {
                    let mut wizard = self.cast_wizard.take().expect("wizard active");
                    wizard.kicked = answer;
                    wizard.stage = cast_wizard::WizardStage::PitchChoice;
                    self.cast_wizard = Some(wizard);
                    return self.advance_cast_wizard();
                }
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
                // Wizard path: pitch cards (exile-from-hand costs).
                if self
                    .cast_wizard
                    .as_ref()
                    .is_some_and(|w| w.stage == cast_wizard::WizardStage::PitchChoice)
                {
                    let mut wizard = self.cast_wizard.take().expect("wizard active");
                    wizard.pitch = objects.into_iter().collect();
                    wizard.stage = cast_wizard::WizardStage::Done;
                    self.cast_wizard = Some(wizard);
                    return self.advance_cast_wizard();
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
