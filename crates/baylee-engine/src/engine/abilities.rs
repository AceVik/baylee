use super::{
    AbilityDef, ActivationTiming, AbilityLoc, CardLookup, Cause, Cost, CostPart, Engine,
    EngineError, GameEvent, GameObject, LegalActions, NameRef, ObjectId, Pending, Phase, PlanKind,
    Resolution, SmallVec, Status, TypeSet, Zone, ZoneLocation, ZonePosition, casting, eval,
    mana_pay, resolve, PlayerId,
};
use baylee_cards_dsl::ActivationZone;

impl<L: CardLookup> Engine<L> {
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
            if obj.characteristics().types.contains(TypeSet::LAND)
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
            for (i, ability) in def.abilities.iter().enumerate() {
                let AbilityDef::Activated {
                    cost, timing, zone, ..
                } = ability else {
                    continue;
                };
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
        }
        // Hand-zone activations (cycling).
        for &card in self.state.zones.list(ZoneLocation::Hand(player)) {
            let Some(obj) = self.state.object(card) else {
                continue;
            };
            let Some(card_ref) = obj.card else { continue };
            let Some(def) = self.lookup.card(card_ref.index) else {
                continue;
            };
            for (i, ability) in def.abilities.iter().enumerate() {
                let AbilityDef::Activated {
                    cost, timing, zone, ..
                } = ability else {
                    continue;
                };
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

    pub(crate) fn start_activation(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        ability_index: u32,
        targets: SmallVec<[ObjectId; 2]>,
    ) -> Result<(), EngineError> {
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
            return Err(EngineError::IllegalAction("ability not usable from this zone"));
        }
        let _ = card_index;
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
