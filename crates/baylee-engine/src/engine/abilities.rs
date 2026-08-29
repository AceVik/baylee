use super::{
    AbilityDef, AbilityLoc, ActivationTiming, CardLookup, Cause, Cost, CostPart, Engine,
    EngineError, GameEvent, GameObject, LegalActions, NameRef, ObjectId, Pending, Phase, PlanKind,
    PlayerId, Resolution, SmallVec, Status, TypeSet, Zone, ZoneLocation, ZonePosition, casting,
    eval, mana_pay, resolve,
};
use baylee_cards_dsl::ActivationZone;

impl<L: CardLookup> Engine<L> {
    #[allow(clippy::too_many_lines)]
    pub(crate) fn compute_legal(&self, player: PlayerId) -> LegalActions {
        let mut legal = LegalActions {
            can_pass: true,
            ..LegalActions::default()
        };
        let main_phase = matches!(self.state.turn.phase, Phase::FirstMain | Phase::SecondMain);
        let sorcery_timing =
            main_phase && self.state.turn.active == player && self.state.zones.stack_is_empty();
        for &card in self.state.zones.list(ZoneLocation::Hand(player)) {
            let Some(obj) = self.state.object(card) else {
                continue;
            };
            let any_face_is_land = if obj.characteristics().types.contains(TypeSet::LAND) {
                true
            } else {
                // MDFC: a back land face is playable (CR 712.4a).
                obj.card
                    .and_then(|c| self.lookup.card(c.index))
                    .is_some_and(|def| def.faces.iter().any(|f| f.types.contains(TypeSet::LAND)))
            };
            if any_face_is_land
                && sorcery_timing
                && self.state.players[player.get() as usize].lands_played_this_turn == 0
            {
                legal.lands.push(card);
            }
            if casting::can_cast(&self.state, &self.lookup, player, card).is_ok() {
                legal.castable.push(card);
            }
        }
        for &id in self.state.zones.list(ZoneLocation::Battlefield) {
            if casting::can_activate_mana(&self.state, player, id) {
                legal.mana_abilities.push(id);
            }
            // Activated abilities of controlled permanents.
            let Some(obj) = self.state.object(id) else {
                continue;
            };
            if obj.controller != player {
                continue;
            }
            let Some(card) = obj.card else { continue };
            let Some(def) = self.lookup.card(card.index) else {
                continue;
            };
            for (i, ability) in def
                .abilities_for_face(obj.face_index as usize)
                .iter()
                .enumerate()
            {
                match ability {
                    AbilityDef::Activated {
                        cost, timing, zone, ..
                    } => {
                        if *zone != ActivationZone::Battlefield {
                            continue; // hand-zone abilities are scanned below
                        }
                        if *timing == ActivationTiming::SorcerySpeed && !sorcery_timing {
                            continue;
                        }
                        if self.can_afford(player, id, cost) {
                            legal.abilities.push((id, i as u32));
                        }
                    }
                    AbilityDef::Loyalty { cost, .. } => {
                        // Loyalty abilities: sorcery timing, once per turn
                        // per walker, enough loyalty for negative costs.
                        if !sorcery_timing || self.loyalty_used_this_turn.contains(&id) {
                            continue;
                        }
                        let loyalty = obj.counters.get(baylee_cards_dsl::CounterKind::Loyalty);
                        if *cost < 0 && loyalty < (-*cost) as u16 {
                            continue;
                        }
                        legal.abilities.push((id, i as u32));
                    }
                    _ => {}
                }
            }
        }
        // Hand-zone activations (cycling) and suspensions.
        for &card in self.state.zones.list(ZoneLocation::Hand(player)) {
            let Some(obj) = self.state.object(card) else {
                continue;
            };
            let Some(card_ref) = obj.card else { continue };
            let Some(def) = self.lookup.card(card_ref.index) else {
                continue;
            };
            for (i, ability) in def
                .abilities_for_face(obj.face_index as usize)
                .iter()
                .enumerate()
            {
                match ability {
                    AbilityDef::Activated {
                        cost, timing, zone, ..
                    } => {
                        if *zone != ActivationZone::Hand {
                            continue;
                        }
                        if *timing == ActivationTiming::SorcerySpeed && !sorcery_timing {
                            continue;
                        }
                        if self.can_afford(player, card, cost) {
                            legal.abilities.push((card, i as u32));
                        }
                    }
                    AbilityDef::Suspend { .. } if sorcery_timing => {
                        legal.suspendable.push(card);
                    }
                    _ => {}
                }
            }
        }
        legal
    }

    pub(crate) fn can_afford(&self, player: PlayerId, source: ObjectId, cost: &Cost) -> bool {
        let pool = &self.state.players[player.get() as usize].mana_pool;
        if !mana_pay::can_pay(pool, &cost.mana) {
            return false;
        }
        for part in cost.parts {
            match part {
                CostPart::TapSelf => {
                    let Some(obj) = self.state.object(source) else {
                        return false;
                    };
                    if obj.status.contains(Status::TAPPED) {
                        return false;
                    }
                }
                CostPart::PayLife(n) => {
                    if self.state.players[player.get() as usize].life <= i32::from(*n) {
                        return false;
                    }
                }
                CostPart::UntapSelf
                | CostPart::SacrificeSelf
                | CostPart::Sacrifice(_)
                | CostPart::Discard(_)
                | CostPart::DiscardSelf
                | CostPart::ExileSelf
                | CostPart::ExileFromHand(_)
                | CostPart::PayLifeX => {}
            }
        }
        true
    }

    // ---------------------------------------------------- S3: abilities

    #[allow(clippy::too_many_lines)] // activation is a staged checklist; extraction would obscure it
    pub(crate) fn start_activation(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        ability_index: u32,
        targets: SmallVec<[ObjectId; 2]>,
    ) -> Result<(), EngineError> {
        // Loyalty abilities route to their own activation path first.
        if let Some(AbilityDef::Loyalty { cost, .. }) = self
            .state
            .object(source)
            .and_then(|o| {
                let face = o.face_index as usize;
                o.card
                    .and_then(|c| self.lookup.card(c.index))
                    .map(|def| def.abilities_for_face(face))
            })
            .and_then(|abilities| abilities.get(ability_index as usize))
        {
            return self.start_loyalty_activation(player, source, ability_index, targets, *cost);
        }
        let (card_index, cost, effects, target, mana_ability, zone) = {
            let obj = self
                .state
                .object(source)
                .ok_or(EngineError::IllegalAction("no such permanent"))?;
            let card = obj
                .card
                .ok_or(EngineError::IllegalAction("not a card-backed object"))?;
            let def = self
                .lookup
                .card(card.index)
                .ok_or(EngineError::IllegalAction("unknown card"))?;
            // Karn's lock: opponents can't activate artifact abilities.
            if obj
                .characteristics()
                .types
                .contains(baylee_core::types::TypeSet::ARTIFACT)
                && self.state.effects.iter().any(|fx| {
                    matches!(
                        fx.modifier,
                        baylee_cards_dsl::Modifier::CantActivateArtifacts
                    ) && fx.controller != obj.controller
                })
            {
                return Err(EngineError::IllegalAction(
                    "activated abilities of artifacts can't be activated (Karn)",
                ));
            }
            let AbilityDef::Activated {
                cost,
                effects,
                target,
                mana_ability,
                zone,
                ..
            } = def
                .abilities
                .get(ability_index as usize)
                .ok_or(EngineError::IllegalAction("no such ability"))?
            else {
                return Err(EngineError::IllegalAction("not an activated ability"));
            };
            (card.index, *cost, *effects, *target, *mana_ability, *zone)
        };
        // Zone validation (battlefield abilities vs. hand abilities).
        let in_right_zone = match zone {
            ActivationZone::Battlefield => self
                .state
                .object(source)
                .is_some_and(|o| o.zone == Zone::Battlefield),
            ActivationZone::Hand => self
                .state
                .object(source)
                .is_some_and(|o| o.zone == Zone::Hand && o.zone_owner == Some(player)),
        };
        if !in_right_zone {
            return Err(EngineError::IllegalAction(
                "ability not usable from this zone",
            ));
        }
        let _ = card_index;
        let _ = zone;
        // Targets (chosen first unless already provided).
        if targets.is_empty()
            && let Some(spec) = target
        {
            let options = eval::target_options(&spec, &self.state, player, source);
            if options.is_empty() {
                return Err(EngineError::IllegalAction("no legal targets"));
            }
            self.pending_plan = Some(PlanKind::ActivateAbility {
                source,
                ability_index,
            });
            self.pending = Pending::ChooseTargets {
                player,
                options,
                min: 1,
                max: 1,
            };
            self.awaiting_answer = true;
            return Ok(());
        }
        if !self.can_afford(player, source, &cost) {
            return Err(EngineError::IllegalAction("cannot pay the cost"));
        }
        self.pay_cost(player, source, &cost)?;
        // Targets are chosen after the cost is paid (CR 602.1b); the
        // target-choice plan completes via finish_activation.
        if targets.is_empty()
            && let Some(spec) = target
        {
            let options = eval::target_options(&spec, &self.state, player, source);
            if options.is_empty() {
                return Err(EngineError::IllegalAction("no legal targets"));
            }
            self.pending_plan = Some(PlanKind::ActivateAbility {
                source,
                ability_index,
            });
            self.pending = Pending::ChooseTargets {
                player,
                options,
                min: 1,
                max: 1,
            };
            self.awaiting_answer = true;
            return Ok(());
        }
        if mana_ability {
            // Mana abilities resolve immediately, without the stack (CR 605.3b).
            let mut res = Resolution {
                source,
                on_stack: source,
                controller: player,
                effects: resolve::flatten(effects),
                pc: 0,
                targets,
                x: None,
                chosen_player: None,
                event_object: None,
                awaiting: None,
            };
            debug_assert!(matches!(
                resolve::run(&mut self.state, &mut res),
                resolve::Flow::Complete
            ));
        } else {
            self.push_ability_to_stack(player, source, ability_index, targets)?;
        }
        self.after_action(player);
        Ok(())
    }

    /// Completes a loyalty activation after targeting: pushes the ability
    /// to the stack without re-paying (cost was paid at activation).
    pub(crate) fn finish_loyalty_activation(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        ability_index: u32,
        targets: SmallVec<[ObjectId; 2]>,
    ) -> Result<(), EngineError> {
        let card_index = self
            .state
            .object(source)
            .and_then(|o| o.card)
            .map(|c| c.index)
            .ok_or(EngineError::IllegalAction("not a card-backed object"))?;
        let loc = AbilityLoc {
            card: card_index,
            index: ability_index,
            source,
        };
        let name = self
            .state
            .object(source)
            .map_or(NameRef::new(0), |o| o.base.name);
        let id = self.state.arena.insert_with(|id| {
            let mut obj = GameObject::new_ability_on_stack(id, player, loc, targets, name);
            obj.chosen_player = self.loyalty_player_choice.take();
            obj
        });
        self.state
            .zones
            .insert(id, ZoneLocation::Stack, ZonePosition::Top);
        self.state.journal.record(GameEvent::AbilityTriggered {
            object: id,
            source,
            ability_index,
            controller: player,
        });
        self.after_action(player);
        Ok(())
    }

    /// Activates a planeswalker loyalty ability: applies the loyalty cost
    /// and puts the ability on the stack (CR 606.2-606.4).
    #[allow(clippy::too_many_lines)] // loyalty activation is a staged checklist
    pub(crate) fn start_loyalty_activation(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        ability_index: u32,
        targets: SmallVec<[ObjectId; 2]>,
        cost: i8,
    ) -> Result<(), EngineError> {
        if self.loyalty_used_this_turn.contains(&source) {
            return Err(EngineError::IllegalAction("loyalty already used this turn"));
        }
        let (card_index, effects, target) = {
            let obj = self
                .state
                .object(source)
                .ok_or(EngineError::IllegalAction("no such permanent"))?;
            let card = obj
                .card
                .ok_or(EngineError::IllegalAction("not a card-backed object"))?;
            let def = self
                .lookup
                .card(card.index)
                .ok_or(EngineError::IllegalAction("unknown card"))?;
            let AbilityDef::Loyalty {
                effects, target, ..
            } = def
                .abilities
                .get(ability_index as usize)
                .ok_or(EngineError::IllegalAction("no such ability"))?
            else {
                return Err(EngineError::IllegalAction("not a loyalty ability"));
            };
            (card.index, *effects, *target)
        };
        // Loyalty cost is paid at activation (CR 606.3) — after checking
        // that required targets exist, before targeting.
        let old = self.state.object(source).map_or(0, |o| {
            o.counters.get(baylee_cards_dsl::CounterKind::Loyalty)
        });
        if cost < 0 && old < (-cost) as u16 {
            return Err(EngineError::IllegalAction("not enough loyalty"));
        }
        if let Some(spec) = target
            && !matches!(spec, baylee_cards_dsl::TargetSpec::AnyPlayer)
        {
            let options = eval::target_options(&spec, &self.state, player, source);
            if options.is_empty() {
                return Err(EngineError::IllegalAction("no legal targets"));
            }
        }
        let new = if cost >= 0 {
            old.saturating_add(cost as u16)
        } else {
            old - (-cost) as u16
        };
        {
            let obj = self.state.object_mut(source).expect("walker exists");
            obj.counters
                .set(baylee_cards_dsl::CounterKind::Loyalty, new);
        }
        self.state.journal.record(GameEvent::CounterChanged {
            object: source,
            kind: baylee_cards_dsl::CounterKind::Loyalty,
            old,
            new,
        });
        self.loyalty_used_this_turn.push(source);
        // Targets first if required.
        if targets.is_empty()
            && let Some(spec) = target
        {
            if matches!(spec, baylee_cards_dsl::TargetSpec::AnyPlayer) {
                let options: Vec<PlayerId> = self
                    .state
                    .players
                    .iter()
                    .filter(|p| !p.has_lost)
                    .map(|p| p.id)
                    .collect();
                self.pending_plan = Some(PlanKind::LoyaltyPlayer {
                    source,
                    ability_index,
                });
                self.pending = Pending::ChoosePlayer { player, options };
                self.awaiting_answer = true;
                return Ok(());
            }
            let options = eval::target_options(&spec, &self.state, player, source);
            if options.is_empty() {
                return Err(EngineError::IllegalAction("no legal targets"));
            }
            self.pending_plan = Some(PlanKind::ActivateAbility {
                source,
                ability_index,
            });
            self.pending = Pending::ChooseTargets {
                player,
                options,
                min: 1,
                max: 1,
            };
            self.awaiting_answer = true;
            return Ok(());
        }
        // The ability goes on the stack.
        let loc = AbilityLoc {
            card: card_index,
            index: ability_index,
            source,
        };
        let name = self
            .state
            .object(source)
            .map_or(NameRef::new(0), |o| o.base.name);
        let id = self.state.arena.insert_with(|id| {
            let mut obj = GameObject::new_ability_on_stack(id, player, loc, targets, name);
            obj.chosen_player = self.loyalty_player_choice.take();
            obj
        });
        self.state
            .zones
            .insert(id, ZoneLocation::Stack, ZonePosition::Top);
        self.state.journal.record(GameEvent::AbilityTriggered {
            object: id,
            source,
            ability_index,
            controller: player,
        });
        self.after_action(player);
        let _ = effects;
        Ok(())
    }

    pub(crate) fn pay_cost(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        cost: &Cost,
    ) -> Result<(), EngineError> {
        if !cost.mana.is_empty() {
            let pool = &mut self.state.players[player.get() as usize].mana_pool;
            if !mana_pay::pay(pool, &cost.mana) {
                return Err(EngineError::IllegalAction("not enough mana"));
            }
        }
        for part in cost.parts {
            match part {
                CostPart::TapSelf => {
                    if let Some(obj) = self.state.object_mut(source) {
                        obj.status.insert(Status::TAPPED);
                    }
                    self.state.journal.record(GameEvent::ObjectTapped {
                        object: source,
                        cause: Cause::Cost,
                    });
                }
                CostPart::UntapSelf => {
                    if let Some(obj) = self.state.object_mut(source) {
                        obj.status.remove(Status::TAPPED);
                    }
                }
                CostPart::DiscardSelf => {
                    let owner = self.state.object(source).map_or(player, |o| o.owner);
                    self.state.move_object(
                        source,
                        ZoneLocation::Graveyard(owner),
                        ZonePosition::Top,
                        Cause::Cost,
                    )?;
                }
                CostPart::SacrificeSelf => {
                    let owner = self.state.object(source).map_or(player, |o| o.owner);
                    self.state.move_object(
                        source,
                        ZoneLocation::Graveyard(owner),
                        ZonePosition::Top,
                        Cause::Cost,
                    )?;
                }
                CostPart::PayLife(n) => {
                    let p = &mut self.state.players[player.get() as usize];
                    let old = p.life;
                    p.life -= i32::from(*n);
                    let new = p.life;
                    self.state.journal.record(GameEvent::LifeChanged {
                        player,
                        old,
                        new,
                        cause: Cause::Cost,
                    });
                }
                CostPart::ExileSelf => {
                    let owner = self.state.object(source).map_or(player, |o| o.owner);
                    self.state.move_object(
                        source,
                        ZoneLocation::Exile(owner),
                        ZonePosition::Top,
                        Cause::Cost,
                    )?;
                }
                CostPart::ExileFromHand(_) | CostPart::PayLifeX => {
                    // Choice/X-driven parts are paid in the casting wizard.
                }
                CostPart::Sacrifice(_) | CostPart::Discard(_) => {
                    return Err(EngineError::IllegalAction(
                        "choice costs are not supported yet (M2)",
                    ));
                }
            }
        }
        Ok(())
    }

    pub(crate) fn push_ability_to_stack(
        &mut self,
        controller: PlayerId,
        source: ObjectId,
        ability_index: u32,
        targets: SmallVec<[ObjectId; 2]>,
    ) -> Result<(), EngineError> {
        let card = self
            .state
            .object(source)
            .and_then(|o| o.card)
            .ok_or(EngineError::IllegalAction("source is not card-backed"))?;
        let name = self
            .state
            .object(source)
            .map_or(NameRef::new(0), |o| o.base.name);
        let id = self.state.arena.insert_with(|id| {
            GameObject::new_ability_on_stack(
                id,
                controller,
                AbilityLoc {
                    card: card.index,
                    index: ability_index,
                    source,
                },
                targets,
                name,
            )
        });
        self.state
            .zones
            .insert(id, ZoneLocation::Stack, ZonePosition::Top);
        self.state.journal.record(GameEvent::AbilityTriggered {
            object: id,
            source,
            ability_index,
            controller,
        });
        Ok(())
    }
}
