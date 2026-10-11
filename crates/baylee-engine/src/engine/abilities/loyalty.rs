//! Loyalty activation choices and payment.
#[allow(clippy::wildcard_imports)] // shared ability vocabulary
use super::*;

impl<L: CardLookup> Engine<L> {
    /// Completes a loyalty activation after targeting: pushes the ability
    /// to the stack without re-paying (cost was paid at activation).
    ///
    /// The one thing it could refuse — a source with no card — was never
    /// its to refuse: the ability came off the object's own list, which is
    /// also the list `legal_actions` offered from. With that gone there is
    /// nothing left here that can fail.
    pub(crate) fn finish_loyalty_activation(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        ability_index: u32,
        targets: SmallVec<[ObjectId; 2]>,
        second: SmallVec<[ObjectId; 1]>,
    ) {
        // A loyalty ability of a card-less permanent is a token copy of a
        // planeswalker: nothing in the pool makes one, but `legal_actions`
        // offers whatever the object's own list holds, so refusing here
        // would be the engine taking back an offer it had just made.
        let card_index = self
            .state
            .object(source)
            .and_then(|o| o.card)
            .map(|c| c.index);
        let loc = AbilityLoc {
            card: card_index,
            index: ability_index,
            source,
        };
        let name = self
            .state
            .object(source)
            .map_or(NameRef::new(0), |o| o.base.name);
        let base = self.state.bare_base(name);
        // CR 608.2, as in `push_ability_to_stack`. A loyalty cost never moves
        // the walker, so the source is still there to be read.
        let abilities = self
            .state
            .object(source)
            .map_or(crate::object::AbilityList::NONE, |o| {
                o.ability_list(&self.lookup)
            });
        let second_req = self.loyalty_second_targets(source, ability_index);
        let id = self.state.arena.insert_with(|id| {
            let mut obj = GameObject::new_ability_on_stack(id, player, loc, targets, base);
            obj.take_abilities(abilities);
            obj.set_second(second, second_req);
            obj.chosen_player = self.loyalty_player_choice.take();
            obj
        });
        self.state
            .zones
            .insert(id, ZoneLocation::Stack, ZonePosition::Top, false);
        self.state
            .bind_target_references(id, &self.activation_target_references);
        self.activation_target_references = crate::sources::TargetReferences::default();
        self.state.journal.record(GameEvent::AbilityTriggered {
            object: id,
            source,
            ability_index,
            controller: player,
        });
        self.after_action(player);
    }

    /// The loyalty cost of `source`'s ability at `ability_index` (CR 606.4),
    /// or `None` when that ability is not a loyalty ability.
    ///
    /// The one probe for "is this a loyalty activation", read off the
    /// object's own list: the list the offer indexed, and the one a copy's
    /// abilities are in (CR 707.2). Starting an activation and finishing it
    /// after its target answer used to ask two different lists. The answer
    /// read the card's printed one, so a Spark Double that had entered as a
    /// copy of Karn had a loyalty ability when it was started and an
    /// ordinary one when it was answered. The answer was then sent into a
    /// second activation, which refused it, and the target question was
    /// left with nothing to answer it.
    pub(crate) fn loyalty_cost(&self, source: ObjectId, ability_index: u32) -> Option<i8> {
        match self
            .state
            .object(source)?
            .abilities(&self.lookup)
            .get(ability_index as usize)?
        {
            AbilityDef::Loyalty { cost, .. } => Some(*cost),
            _ => None,
        }
    }

    /// The second instance of "target" on a loyalty ability, if it prints
    /// one; `None` as well for an ability that is not a loyalty ability.
    pub(crate) fn loyalty_second_targets(
        &self,
        source: ObjectId,
        ability_index: u32,
    ) -> Option<baylee_cards_dsl::TargetReq> {
        match self
            .state
            .object(source)?
            .abilities(&self.lookup)
            .get(ability_index as usize)?
        {
            AbilityDef::Loyalty { second_targets, .. } => *second_targets,
            _ => None,
        }
    }

    /// A loyalty ability whose first instance of "target" has its answer:
    /// asks for the second (Oko, Thief of Crowns' −5) the way an activated
    /// ability does (CR 601.2c, CR 115.3), or puts it on the stack. The
    /// loyalty is already paid (CR 606.4), and `start_loyalty_activation`
    /// made sure before paying that the second instance can be answered.
    pub(crate) fn continue_loyalty_activation(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        ability_index: u32,
        targets: SmallVec<[ObjectId; 2]>,
    ) {
        if let Some(req) = self.loyalty_second_targets(source, ability_index) {
            let options = eval::target_options_with_context(
                &req.spec,
                &self.state,
                player,
                self.ability_rule_context(source, ability_index),
            );
            if !options.is_empty() && req.max > 0 {
                self.pending_plan = Some(PlanKind::ActivateAbilitySecondTargets {
                    source,
                    ability_index,
                    targets,
                    target_players: Vec::new(),
                });
                let (min, max) = req.bounds(self.activation_x.unwrap_or(0));
                let max = max.min(u32::try_from(options.len()).unwrap_or(u32::MAX));
                self.pending = Pending::ChooseTargets {
                    player,
                    options,
                    player_options: Vec::new(),
                    min,
                    max,
                    reason: TargetPrompt::Targets,
                };
                self.awaiting_answer = true;
                return;
            }
        }
        self.finish_loyalty_activation(player, source, ability_index, targets, SmallVec::new());
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
        let wanted = {
            let obj = self
                .state
                .object(source)
                .ok_or(EngineError::IllegalAction("no such permanent"))?;
            // The ability comes off the object's own list: a token copy of a
            // planeswalker has one without having a card.
            let AbilityDef::Loyalty { targets, .. } = obj
                .abilities(&self.lookup)
                .get(ability_index as usize)
                .ok_or(EngineError::IllegalAction("no such ability"))?
            else {
                return Err(EngineError::IllegalAction("not a loyalty ability"));
            };
            *targets
        };
        // Loyalty cost is paid at activation (CR 606.4) — after checking
        // that required targets exist, before targeting.
        let old = self.state.object(source).map_or(0, |o| {
            o.counters.get(baylee_cards_dsl::CounterKind::Loyalty)
        });
        if cost < 0 && old < (-cost) as u16 {
            return Err(EngineError::IllegalAction("not enough loyalty"));
        }
        // Counted as the questions below count them, a player spec in
        // players and every other in objects, so that no refusal waits past
        // the payment: loyalty paid and a refusal after it is a changed
        // engine the record never sees.
        for req in [wanted, self.loyalty_second_targets(source, ability_index)]
            .into_iter()
            .flatten()
        {
            let found = if matches!(
                req.spec,
                baylee_cards_dsl::TargetSpec::AnyPlayer | baylee_cards_dsl::TargetSpec::AnyOpponent
            ) {
                eval::target_player_options(&self.state, &req.spec, player).len()
            } else {
                eval::target_options_with_context(
                    &req.spec,
                    &self.state,
                    player,
                    self.ability_rule_context(source, ability_index),
                )
                .len()
            };
            if found < req.min as usize {
                return Err(EngineError::IllegalAction("no legal targets"));
            }
        }
        let price = self.activation_price(player, source, &Cost::FREE, None);
        self.pay_cost(player, source, &price, &[], 0)?;
        if cost >= 0 {
            crate::replacement::record_counters_by(
                &mut self.state,
                player,
                source,
                baylee_cards_dsl::CounterKind::Loyalty,
                cost as u16,
            );
        } else {
            crate::replacement::remove_counters(
                &mut self.state,
                source,
                baylee_cards_dsl::CounterKind::Loyalty,
                (-cost) as u16,
            );
        }
        self.loyalty_used_this_turn.push(source);
        // Targets first if required.
        if targets.is_empty()
            && let Some(req) = wanted
        {
            if matches!(
                req.spec,
                baylee_cards_dsl::TargetSpec::AnyPlayer | baylee_cards_dsl::TargetSpec::AnyOpponent
            ) {
                let options = eval::target_player_options(&self.state, &req.spec, player);
                if options.len() < req.min as usize {
                    return Err(EngineError::IllegalAction("no legal targets"));
                }
                self.pending_plan = Some(PlanKind::LoyaltyPlayer {
                    source,
                    ability_index,
                });
                self.pending = Pending::ChoosePlayer { player, options };
                self.awaiting_answer = true;
                return Ok(());
            }
            let options = eval::target_options_with_context(
                &req.spec,
                &self.state,
                player,
                self.ability_rule_context(source, ability_index),
            );
            if options.len() < req.min as usize {
                return Err(EngineError::IllegalAction("no legal targets"));
            }
            // "Up to one target" with nothing on the board to point at is not
            // a question: there would be one answer to it. The ability goes
            // on the stack targeting nothing, which is what the printing says
            // it may do.
            if !options.is_empty() {
                self.pending_plan = Some(PlanKind::ActivateAbility {
                    source,
                    ability_index,
                });
                let (min, max) = req.bounds(self.activation_x.unwrap_or(0));
                let max = max.min(u32::try_from(options.len()).unwrap_or(u32::MAX));
                self.pending = Pending::ChooseTargets {
                    player,
                    options,
                    player_options: Vec::new(),
                    min,
                    max,
                    reason: TargetPrompt::Targets,
                };
                self.awaiting_answer = true;
                return Ok(());
            }
        }
        self.continue_loyalty_activation(player, source, ability_index, targets);
        Ok(())
    }
}
