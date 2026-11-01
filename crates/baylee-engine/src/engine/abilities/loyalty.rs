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
        let second_req = self.loyalty_second_targets(source, ability_index);
        let id = if let Some(grant) = self.granted_definition(source, ability_index) {
            self.push_granted_ability(player, source, targets, &grant)
        } else {
            self.push_ability_to_stack(player, source, ability_index, targets)
        };
        if let Some(object) = self.state.object_mut(id) {
            object.set_second(second, second_req);
            object.chosen_player = self.loyalty_player_choice.take();
            object.paid = self.loyalty_paid.take().map(Box::new);
        }
        self.apply_ability_spend_riders(player, id);
        self.activating_grant = None;
        self.state
            .bind_target_references(id, &self.activation_target_references);
        self.activation_target_references = crate::sources::TargetReferences::default();
        self.state.journal.record(GameEvent::AbilityActivated {
            ability: self.state.source_identity(id),
            source: self.state.source_identity(source).expect("loyalty source"),
            player,
            tap_cost: false,
            mana_ability: false,
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
        match self.activation_definition(source, ability_index)? {
            AbilityDef::Loyalty { cost, .. } => Some(cost),
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
        match self.activation_definition(source, ability_index)? {
            AbilityDef::Loyalty { second_targets, .. } => second_targets,
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
            let options = eval::target_options_in_domain(
                &req.spec,
                &self.state,
                player,
                self.ability_rule_context(source, ability_index),
                None,
                self.activation_wall_only(source, ability_index),
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
        let Some(AbilityDef::Loyalty {
            targets: wanted, ..
        }) = self.activation_definition(source, ability_index)
        else {
            return Err(EngineError::IllegalAction("not a loyalty ability"));
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
                baylee_cards_dsl::TargetSpec::AnyPlayer
                    | baylee_cards_dsl::TargetSpec::AnyOpponent
                    | baylee_cards_dsl::TargetSpec::OpponentWithMoreThanActivePlayer(_)
                    | baylee_cards_dsl::TargetSpec::OpponentWithMoreThanPlayer(..)
            ) {
                eval::target_player_options_from(
                    &self.state,
                    &req.spec,
                    player,
                    self.ability_rule_context(source, ability_index),
                )
                .len()
            } else {
                eval::target_options_in_domain(
                    &req.spec,
                    &self.state,
                    player,
                    self.ability_rule_context(source, ability_index),
                    None,
                    self.activation_wall_only(source, ability_index),
                )
                .len()
            };
            if found < req.min as usize {
                return Err(EngineError::IllegalAction("no legal targets"));
            }
        }
        let price = self.activation_price(player, source, ability_index, &Cost::FREE, None);
        self.loyalty_paid = Some(self.pay_priced_cost(player, source, &price, &[], 0)?);
        if cost >= 0 {
            crate::replacement::record_counters_by(
                &mut self.state,
                player,
                source,
                baylee_cards_dsl::CounterKind::Loyalty,
                cost as u16,
            );
        } else {
            // A cost its activator pays: they remove the counters.
            crate::replacement::remove_counters_by(
                &mut self.state,
                source,
                baylee_cards_dsl::CounterKind::Loyalty,
                (-cost) as u16,
                Some(player),
            );
        }
        if crate::replacement::counter_order::has_pending(&self.state) {
            self.activation_payment = Some(super::payment::ActivationPayment::new(
                super::payment::ActivationPaymentKind::Loyalty {
                    player,
                    source,
                    ability_index,
                    targets,
                    wanted,
                },
            ));
            self.offer_counter_order();
            return Ok(());
        }
        self.complete_loyalty_payment(player, source, ability_index, targets, wanted)
    }

    pub(super) fn complete_loyalty_payment(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        ability_index: u32,
        targets: SmallVec<[ObjectId; 2]>,
        wanted: Option<baylee_cards_dsl::TargetReq>,
    ) -> Result<(), EngineError> {
        self.loyalty_used_this_turn.push(source);
        // Targets first if required.
        if targets.is_empty()
            && let Some(req) = wanted
        {
            if matches!(
                req.spec,
                baylee_cards_dsl::TargetSpec::AnyPlayer
                    | baylee_cards_dsl::TargetSpec::AnyOpponent
                    | baylee_cards_dsl::TargetSpec::OpponentWithMoreThanActivePlayer(_)
                    | baylee_cards_dsl::TargetSpec::OpponentWithMoreThanPlayer(..)
            ) {
                let options = eval::target_player_options_from(
                    &self.state,
                    &req.spec,
                    player,
                    self.ability_rule_context(source, ability_index),
                );
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
            let options = eval::target_options_in_domain(
                &req.spec,
                &self.state,
                player,
                self.ability_rule_context(source, ability_index),
                None,
                self.activation_wall_only(source, ability_index),
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
