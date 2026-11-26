//! Loyalty activation choices and payment.
#[allow(clippy::wildcard_imports)] // shared ability vocabulary
use super::*;

impl<L: CardLookup, D: crate::engine::LoopDetection> Engine<L, D> {
    /// Whether `player` may activate loyalty abilities of `source` at instant
    /// speed on any player's turn (`CR 606.3`, *Teferi's Talent* emblem).
    pub(crate) fn may_activate_loyalty_as_instant(
        &self,
        player: PlayerId,
        source: ObjectId,
    ) -> bool {
        let Some(obj) = self.state.object(source) else {
            return false;
        };
        self.state.effects.iter().any(|fx| {
            let baylee_cards_dsl::Modifier::MayActivateLoyaltyAsInstant { who } = fx.modifier
            else {
                return false;
            };
            crate::eval::players_for_source(who, &self.state, fx.controller, fx.source)
                .is_some_and(|players| players.contains(&player))
                && crate::effects::applies_to(&self.state, fx, obj)
        })
    }

    /// Whether this incarnation of `source` already activated a loyalty
    /// ability this turn. A permanent that left and came back is a new
    /// object (CR 400.7) and starts over (CR 606.3).
    pub(crate) fn loyalty_used(&self, source: ObjectId) -> bool {
        let Some(object) = self.state.object(source) else {
            return false;
        };
        self.loyalty_used_this_turn
            .iter()
            .any(|used| used.object == source && used.version == object.version)
    }

    /// Records that this incarnation of `source` used its loyalty ability.
    pub(crate) fn loyalty_used_mark(&mut self, source: ObjectId) {
        if let Some(version) = self.state.object(source).map(|o| o.version) {
            self.loyalty_used_this_turn
                .push(baylee_core::ids::DamageSourceRef {
                    object: source,
                    version,
                });
        }
    }

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
        let third_req = self.loyalty_third_targets(source, ability_index);
        let target_players = std::mem::take(&mut self.activation_target_players);
        let second_players = self.activation_target_choice.second_players;
        let third_targets = self.activation_target_choice.third_targets.take();
        let third_players = self.activation_target_choice.third_players;
        let chosen_loyalty_player = self.loyalty_player_choice.take();
        let id = if let Some(grant) = self.granted_definition(source, ability_index) {
            let mut needs_grantor = false;
            baylee_cards_dsl::Effect::walk(grant.effects, &mut 0, &mut |effect| {
                needs_grantor |=
                    matches!(effect, baylee_cards_dsl::Effect::GrantorDealsDamage { .. });
            });
            let grantor = if needs_grantor {
                grant.source.and_then(|id| self.state.source_identity(id))
            } else {
                None
            };
            self.push_granted_ability(player, source, targets, &grant, grantor)
        } else {
            self.push_ability_to_stack(player, source, ability_index, targets)
        };
        if let Some(object) = self.state.object_mut(id) {
            object.set_second(second, second_req);
            object.set_second_players(second_players);
            if let Some(third) = third_targets {
                object.set_third(third.into_iter().collect(), third_req);
                object.set_third_players(third_players);
            }
            if target_players.is_empty() {
                object.chosen_player = chosen_loyalty_player;
            } else {
                object.target_players = target_players.iter().copied().collect();
                if let [only] = target_players[..] {
                    object.chosen_player = Some(only);
                }
            }
            object.paid = self.loyalty_paid.take().map(Box::new);
        }
        self.apply_ability_spend_riders(player, id);
        let first_groups = std::mem::take(&mut self.activation_target_choice.first_groups);
        let second_groups = std::mem::take(&mut self.activation_target_choice.second_groups);
        let third_groups = std::mem::take(&mut self.activation_target_choice.third_groups);
        self.activation_second_targets = None;
        self.activating_grant = None;
        self.release_target_choice();
        self.activation_targets_answered = false;
        self.state
            .bind_target_references(id, &self.activation_target_references);
        if !first_groups.is_empty() {
            self.state.bind_one_per_player_groups(id, 0, &first_groups);
        }
        if !second_groups.is_empty() {
            self.state.bind_one_per_player_groups(id, 1, &second_groups);
        }
        if !third_groups.is_empty() {
            self.state.bind_one_per_player_groups(id, 2, &third_groups);
        }
        self.activation_target_references = crate::sources::TargetReferences::default();
        self.record_activation_division(id, &[]);
        let loyalty_source = self.state.source_identity(source).expect("loyalty source");
        self.record_activated_ability_this_turn(
            player,
            loyalty_source.object,
            loyalty_source.version,
            Some(id),
        );
        self.state.journal.record(GameEvent::AbilityActivated {
            ability: self.state.source_identity(id),
            source: loyalty_source,
            player,
            tap_cost: false,
            mana_ability: false,
            loyalty: true,
        });
        self.note_crime_if_any(player, id);
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
        let AbilityDef::Loyalty { cost, .. } = self.activation_definition(source, ability_index)?
        else {
            return None;
        };
        if !self.state.effects.may_change_or_forbid_abilities() {
            return Some(cost);
        }
        let Some(object) = self.state.object(source) else {
            return Some(cost);
        };
        let mut adjusted = i16::from(cost);
        for effect in self.state.effects.iter() {
            let baylee_cards_dsl::Modifier::AdjustLoyaltyCost { who, filter, delta } =
                effect.modifier
            else {
                continue;
            };
            if delta == 0 {
                continue;
            }
            let player_ok =
                eval::players_for_source(who, &self.state, effect.controller, effect.source)
                    .is_some_and(|players| players.contains(&object.controller));
            if !player_ok {
                continue;
            }
            let outer_ok = matches!(
                effect.filter,
                crate::effects::EffectFilter::Dsl(
                    baylee_cards_dsl::Filter::Any | baylee_cards_dsl::Filter::This
                ) | crate::effects::EffectFilter::DslControlledBy(_, _)
            ) || crate::effects::applies_to(&self.state, effect, object);
            if !outer_ok {
                continue;
            }
            let context = crate::text_changes::RuleContext {
                source: effect.source.unwrap_or(source),
                text: self.state.effect_text(effect),
                resolved_name: None,
                resolved_subtype: effect.filter.chosen_subtype(),
                reference: None,
            };
            if eval::matches_with_context(filter, &self.state, object, effect.controller, context) {
                adjusted = adjusted.saturating_add(i16::from(delta));
            }
        }
        Some(i8::try_from(adjusted.clamp(i16::from(i8::MIN), i16::from(i8::MAX))).unwrap_or(cost))
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

    /// The third instance of "target" on a loyalty ability, if it prints
    /// one; `None` as well for an ability that is not a loyalty ability.
    pub(crate) fn loyalty_third_targets(
        &self,
        source: ObjectId,
        ability_index: u32,
    ) -> Option<baylee_cards_dsl::TargetReq> {
        match self.activation_definition(source, ability_index)? {
            AbilityDef::Loyalty { third_targets, .. } => third_targets,
            _ => None,
        }
    }

    fn loyalty_distinct_pool(
        &self,
        player: PlayerId,
        source: ObjectId,
        ability_index: u32,
        req: baylee_cards_dsl::TargetReq,
    ) -> eval::DistinctGroupPool {
        let ctx = self.ability_rule_context(source, ability_index);
        let wall_only = self.activation_wall_only(source, ability_index);
        eval::DistinctGroupPool {
            needed: usize::from(req.min),
            distinct_from_prior: req.constraint()
                == Some(baylee_cards_dsl::TargetConstraint::DistinctFromPriorTargetGroups),
            objects: eval::target_options_in_domain(
                &req.spec,
                &self.state,
                player,
                ctx,
                None,
                wall_only,
            ),
            players: eval::target_player_options_from(&self.state, &req.spec, player, ctx),
        }
    }

    fn loyalty_first_players(&self) -> baylee_core::ids::SeatSet {
        let mut seats: baylee_core::ids::SeatSet =
            self.activation_target_players.iter().copied().collect();
        if let Some(p) = self.loyalty_player_choice {
            seats.insert(p);
        }
        seats
    }

    /// A loyalty ability whose first instance of "target" has its answer:
    /// asks for the second (Oko, Thief of Crowns' −5) the way an activated
    /// ability does (CR 601.2c, CR 115.3), or continues to the third / pays
    /// the loyalty cost (CR 601.2h, CR 606.4) and puts the ability on the stack.
    #[allow(clippy::too_many_lines)]
    pub(crate) fn continue_loyalty_activation(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        ability_index: u32,
        targets: SmallVec<[ObjectId; 2]>,
    ) -> Result<(), EngineError> {
        if let Some(req) = self.loyalty_second_targets(source, ability_index) {
            let (min, max) = req.bounds(self.activation_x.unwrap_or(0));
            let ctx = self.ability_rule_context(source, ability_index);
            let mut options = eval::target_options_in_domain(
                &req.spec,
                &self.state,
                player,
                ctx,
                None,
                self.activation_wall_only(source, ability_index),
            );
            let mut player_options =
                eval::target_player_options_from(&self.state, &req.spec, player, ctx);
            let later_pools: Vec<eval::DistinctGroupPool> = self
                .loyalty_third_targets(source, ability_index)
                .into_iter()
                .map(|r3| self.loyalty_distinct_pool(player, source, ability_index, r3))
                .collect();
            eval::retain_distinct_group_options(
                req,
                &mut options,
                &mut player_options,
                &targets,
                self.loyalty_first_players(),
                &later_pools,
            );
            if req.constraint() == Some(baylee_cards_dsl::TargetConstraint::OnePerPlayer) {
                if !eval::one_per_player_satisfied(
                    &self.state,
                    &req.spec,
                    player,
                    &options,
                    req.min,
                ) {
                    return Err(EngineError::IllegalAction("no legal targets"));
                }
                if max > 0 {
                    let seats = eval::one_per_player_seats(&self.state, &req.spec, player);
                    let asking =
                        super::super::PerOpponent::one_per_player(req.spec, seats, req.min, player);
                    let plan = PlanKind::ActivateAbilitySecondTargets {
                        source,
                        ability_index,
                        targets: targets.clone(),
                        target_players: self.activation_target_players.clone(),
                    };
                    if let Some(done) = self.ask_next_activation_per_player(
                        player,
                        self.ability_rule_context(source, ability_index),
                        None,
                        self.activation_wall_only(source, ability_index),
                        asking,
                        plan,
                    ) {
                        self.activation_target_references.second =
                            self.state.capture_target_group(&done.gathered);
                        self.activation_target_choice.second_groups = done.groups;
                        return self.continue_loyalty_third_targets(
                            player,
                            source,
                            ability_index,
                            targets,
                            done.gathered.into_iter().collect(),
                        );
                    }
                    return Ok(());
                }
            } else {
                if req.constraint() == Some(baylee_cards_dsl::TargetConstraint::SameController) {
                    if let Some(chosen) = self.activation_graveyard.take() {
                        options.retain(|id| {
                            eval::same_controller_group(&self.state, *id) == Some(chosen)
                        });
                    } else {
                        let groups =
                            eval::eligible_same_controller_groups(&self.state, &options, min);
                        match groups.as_slice() {
                            [] => options.clear(),
                            [only] => {
                                options.retain(|id| {
                                    eval::same_controller_group(&self.state, *id) == Some(*only)
                                });
                            }
                            _ => {
                                self.pending_plan = Some(PlanKind::ChooseActivationSecondGroup {
                                    source,
                                    ability_index,
                                    targets,
                                    target_players: self.activation_target_players.clone(),
                                });
                                self.pending = Pending::ChoosePlayer {
                                    player,
                                    options: groups,
                                };
                                self.awaiting_answer = true;
                                return Ok(());
                            }
                        }
                    }
                }
                let offered = options.len() + player_options.len();
                if offered < min as usize {
                    return Err(EngineError::IllegalAction("no legal targets"));
                }
                if offered > 0 && max > 0 {
                    self.pending_plan = Some(PlanKind::ActivateAbilitySecondTargets {
                        source,
                        ability_index,
                        targets,
                        target_players: self.activation_target_players.clone(),
                    });
                    let max = max.min(u32::try_from(offered).unwrap_or(u32::MAX));
                    self.pending = Pending::ChooseTargets {
                        player,
                        options,
                        player_options,
                        min,
                        max,
                        reason: TargetPrompt::Targets,
                    };
                    self.awaiting_answer = true;
                    return Ok(());
                }
            }
        }
        self.continue_loyalty_third_targets(player, source, ability_index, targets, SmallVec::new())
    }

    #[allow(clippy::too_many_lines)]
    pub(crate) fn continue_loyalty_third_targets(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        ability_index: u32,
        targets: SmallVec<[ObjectId; 2]>,
        second: SmallVec<[ObjectId; 1]>,
    ) -> Result<(), EngineError> {
        if let Some(req) = self.loyalty_third_targets(source, ability_index) {
            let (min, max) = req.bounds(self.activation_x.unwrap_or(0));
            let ctx = self.ability_rule_context(source, ability_index);
            let mut options = eval::target_options_in_domain(
                &req.spec,
                &self.state,
                player,
                ctx,
                None,
                self.activation_wall_only(source, ability_index),
            );
            let mut player_options =
                eval::target_player_options_from(&self.state, &req.spec, player, ctx);
            let mut prior_objs: Vec<ObjectId> = targets.iter().copied().collect();
            for &id in &second {
                if !prior_objs.contains(&id) {
                    prior_objs.push(id);
                }
            }
            let mut prior_players = self.loyalty_first_players();
            for p in self.activation_target_choice.second_players.iter() {
                prior_players.insert(p);
            }
            eval::retain_distinct_group_options(
                req,
                &mut options,
                &mut player_options,
                &prior_objs,
                prior_players,
                &[],
            );
            if req.constraint() == Some(baylee_cards_dsl::TargetConstraint::OnePerPlayer) {
                if !eval::one_per_player_satisfied(
                    &self.state,
                    &req.spec,
                    player,
                    &options,
                    req.min,
                ) {
                    return Err(EngineError::IllegalAction("no legal targets"));
                }
                if max > 0 {
                    let seats = eval::one_per_player_seats(&self.state, &req.spec, player);
                    let asking =
                        super::super::PerOpponent::one_per_player(req.spec, seats, req.min, player);
                    self.activation_second_targets = Some(second.clone());
                    let plan = PlanKind::ActivateAbilityThirdTargets {
                        source,
                        ability_index,
                        targets: targets.clone(),
                        target_players: self.activation_target_players.clone(),
                    };
                    if let Some(done) = self.ask_next_activation_per_player(
                        player,
                        self.ability_rule_context(source, ability_index),
                        None,
                        self.activation_wall_only(source, ability_index),
                        asking,
                        plan,
                    ) {
                        self.activation_second_targets = None;
                        self.activation_target_references.extra =
                            vec![self.state.capture_target_group(&done.gathered)];
                        self.activation_target_choice.third_groups = done.groups;
                        self.activation_target_choice.third_targets = Some(done.gathered);
                    } else {
                        return Ok(());
                    }
                } else {
                    self.activation_target_choice.third_targets = Some(SmallVec::new());
                }
            } else {
                if req.constraint() == Some(baylee_cards_dsl::TargetConstraint::SameController) {
                    if let Some(chosen) = self.activation_graveyard.take() {
                        options.retain(|id| {
                            eval::same_controller_group(&self.state, *id) == Some(chosen)
                        });
                    } else {
                        let groups =
                            eval::eligible_same_controller_groups(&self.state, &options, min);
                        match groups.as_slice() {
                            [] => options.clear(),
                            [only] => {
                                options.retain(|id| {
                                    eval::same_controller_group(&self.state, *id) == Some(*only)
                                });
                            }
                            _ => {
                                self.activation_second_targets = Some(second);
                                self.pending_plan = Some(PlanKind::ChooseActivationThirdGroup {
                                    source,
                                    ability_index,
                                    targets,
                                    target_players: self.activation_target_players.clone(),
                                });
                                self.pending = Pending::ChoosePlayer {
                                    player,
                                    options: groups,
                                };
                                self.awaiting_answer = true;
                                return Ok(());
                            }
                        }
                    }
                }
                let offered = options.len() + player_options.len();
                if offered < min as usize {
                    return Err(EngineError::IllegalAction("no legal targets"));
                }
                if offered > 0 && max > 0 {
                    self.activation_second_targets = Some(second);
                    self.pending_plan = Some(PlanKind::ActivateAbilityThirdTargets {
                        source,
                        ability_index,
                        targets,
                        target_players: self.activation_target_players.clone(),
                    });
                    let max = max.min(u32::try_from(offered).unwrap_or(u32::MAX));
                    self.pending = Pending::ChooseTargets {
                        player,
                        options,
                        player_options,
                        min,
                        max,
                        reason: TargetPrompt::Targets,
                    };
                    self.awaiting_answer = true;
                    return Ok(());
                }
                self.activation_target_choice.third_targets = Some(SmallVec::new());
            }
        }
        if let Some(AbilityDef::Loyalty { effects, .. }) =
            self.activation_definition(source, ability_index)
            && self.ask_activation_division(
                player,
                source,
                ability_index,
                effects,
                targets.clone(),
                self.activation_target_players.clone(),
            )
        {
            return Ok(());
        }
        self.pay_loyalty_and_finish(player, source, ability_index, targets, second)
    }

    /// Pays a loyalty ability's cost after its targets have been chosen
    /// (CR 602.2b, CR 601.2c–h, CR 606.4), then pushes it to the stack.
    pub(crate) fn pay_loyalty_and_finish(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        ability_index: u32,
        targets: SmallVec<[ObjectId; 2]>,
        second: SmallVec<[ObjectId; 1]>,
    ) -> Result<(), EngineError> {
        let cost = self
            .loyalty_cost(source, ability_index)
            .ok_or(EngineError::IllegalAction("not a loyalty ability"))?;
        let old = self.state.object(source).map_or(0, |o| {
            o.counters.get(baylee_cards_dsl::CounterKind::Loyalty)
        });
        if cost < 0 && old < (-cost) as u16 {
            return Err(EngineError::IllegalAction("not enough loyalty"));
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
        self.loyalty_used_mark(source);
        if crate::replacement::counter_order::has_pending(&self.state) {
            self.activation_payment = Some(super::payment::ActivationPayment::new(
                super::payment::ActivationPaymentKind::Loyalty {
                    player,
                    source,
                    ability_index,
                    targets,
                    second,
                },
            ));
            self.offer_counter_order();
            return Ok(());
        }
        self.finish_loyalty_activation(player, source, ability_index, targets, second);
        Ok(())
    }

    /// Activates a planeswalker loyalty ability: chooses targets first
    /// (CR 602.2b, CR 601.2c), then pays the loyalty cost (CR 601.2h, CR 606.4)
    /// and puts the ability on the stack.
    #[allow(clippy::too_many_lines)] // loyalty activation is a staged checklist
    pub(crate) fn start_loyalty_activation(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        ability_index: u32,
        targets: SmallVec<[ObjectId; 2]>,
        cost: i8,
    ) -> Result<(), EngineError> {
        if self.loyalty_used(source) {
            return Err(EngineError::IllegalAction("loyalty already used this turn"));
        }
        let main_phase = matches!(
            self.state.turn.phase,
            crate::turn::Phase::FirstMain | crate::turn::Phase::SecondMain
        );
        let sorcery_timing =
            main_phase && self.state.turn.active == player && self.state.zones.stack_is_empty();
        if !sorcery_timing && !self.may_activate_loyalty_as_instant(player, source) {
            return Err(EngineError::IllegalAction(
                "loyalty abilities can only be activated as a sorcery",
            ));
        }
        let Some(AbilityDef::Loyalty {
            targets: wanted, ..
        }) = self.activation_definition(source, ability_index)
        else {
            return Err(EngineError::IllegalAction("not a loyalty ability"));
        };
        let old = self.state.object(source).map_or(0, |o| {
            o.counters.get(baylee_cards_dsl::CounterKind::Loyalty)
        });
        if cost < 0 && old < (-cost) as u16 {
            return Err(EngineError::IllegalAction("not enough loyalty"));
        }
        let price = self.activation_price(player, source, ability_index, &Cost::FREE, None);
        if !self.can_pay_activation_price(player, source, &price) {
            return Err(EngineError::IllegalAction("cannot pay the cost"));
        }
        let second_req = self.loyalty_second_targets(source, ability_index);
        let third_req = self.loyalty_third_targets(source, ability_index);
        if !self.ability_has_targets(player, source, ability_index, wanted, second_req, third_req) {
            return Err(EngineError::IllegalAction("no legal targets"));
        }
        // Choose targets first (CR 602.2b -> CR 601.2c) before paying loyalty
        // counters (CR 601.2h, CR 606.4).
        if !self.activation_targets_answered
            && targets.is_empty()
            && self.loyalty_player_choice.is_none()
            && let Some(req) = wanted
        {
            let later_pools: Vec<eval::DistinctGroupPool> = [second_req, third_req]
                .into_iter()
                .flatten()
                .map(|r| self.loyalty_distinct_pool(player, source, ability_index, r))
                .collect();
            if matches!(
                req.spec,
                baylee_cards_dsl::TargetSpec::AnyPlayer
                    | baylee_cards_dsl::TargetSpec::AnyOpponent
                    | baylee_cards_dsl::TargetSpec::OpponentWithMoreThanActivePlayer(_)
                    | baylee_cards_dsl::TargetSpec::OpponentWithMoreThanPlayer(..)
            ) {
                let mut options = eval::target_player_options_from(
                    &self.state,
                    &req.spec,
                    player,
                    self.ability_rule_context(source, ability_index),
                );
                let mut empty_objs = Vec::new();
                eval::retain_distinct_group_options(
                    req,
                    &mut empty_objs,
                    &mut options,
                    &[],
                    baylee_core::ids::SeatSet::new(),
                    &later_pools,
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
            let (min, mut max) = req.bounds(self.activation_x.unwrap_or(0));
            if let Some(AbilityDef::Loyalty { effects, .. }) =
                self.activation_definition(source, ability_index)
                && let Some((_, total)) = crate::engine::division::effects_divided_spec(
                    effects,
                    &self.state,
                    player,
                    self.ability_rule_context(source, ability_index),
                    self.state.object(source).map(|o| o.version),
                    0,
                )
            {
                max = max.min(total);
            }
            if min > max {
                return Err(EngineError::IllegalAction("no legal targets"));
            }
            let ctx = self.ability_rule_context(source, ability_index);
            let mut options = eval::target_options_in_domain(
                &req.spec,
                &self.state,
                player,
                ctx,
                None,
                self.activation_wall_only(source, ability_index),
            );
            let mut player_options =
                eval::target_player_options_from(&self.state, &req.spec, player, ctx);
            if req.constraint() == Some(baylee_cards_dsl::TargetConstraint::OnePerPlayer) {
                if !eval::one_per_player_satisfied(
                    &self.state,
                    &req.spec,
                    player,
                    &options,
                    req.min,
                ) {
                    return Err(EngineError::IllegalAction("no legal targets"));
                }
                if max > 0 {
                    let seats = eval::one_per_player_seats(&self.state, &req.spec, player);
                    let asking =
                        super::super::PerOpponent::one_per_player(req.spec, seats, req.min, player);
                    let plan = PlanKind::ActivateAbility {
                        source,
                        ability_index,
                    };
                    if let Some(done) = self.ask_next_activation_per_player(
                        player,
                        self.ability_rule_context(source, ability_index),
                        None,
                        self.activation_wall_only(source, ability_index),
                        asking,
                        plan,
                    ) {
                        self.activation_target_references.first =
                            self.state.capture_target_group(&done.gathered);
                        self.activation_target_choice.first_groups = done.groups;
                        return self.continue_loyalty_activation(
                            player,
                            source,
                            ability_index,
                            done.gathered,
                        );
                    }
                    return Ok(());
                }
            } else {
                if req.constraint() == Some(baylee_cards_dsl::TargetConstraint::SameController) {
                    if let Some(chosen) = self.activation_graveyard.take() {
                        options.retain(|id| {
                            eval::same_controller_group(&self.state, *id) == Some(chosen)
                        });
                    } else {
                        let groups =
                            eval::eligible_same_controller_groups(&self.state, &options, min);
                        match groups.as_slice() {
                            [] => options.clear(),
                            [only] => {
                                options.retain(|id| {
                                    eval::same_controller_group(&self.state, *id) == Some(*only)
                                });
                            }
                            _ => {
                                self.pending_plan = Some(PlanKind::ChooseActivationGraveyard {
                                    source,
                                    ability_index,
                                });
                                self.pending = Pending::ChoosePlayer {
                                    player,
                                    options: groups,
                                };
                                self.awaiting_answer = true;
                                return Ok(());
                            }
                        }
                    }
                }
                eval::retain_distinct_group_options(
                    req,
                    &mut options,
                    &mut player_options,
                    &[],
                    baylee_core::ids::SeatSet::new(),
                    &later_pools,
                );
                let offered = options.len() + player_options.len();
                if offered < min as usize {
                    return Err(EngineError::IllegalAction("no legal targets"));
                }
                // "Up to one target" with nothing on the board to point at is not
                // a question: there would be one answer to it. The ability goes
                // on the stack targeting nothing, which is what the printing says
                // it may do.
                if offered > 0 && max > 0 {
                    self.pending_plan = Some(PlanKind::ActivateAbility {
                        source,
                        ability_index,
                    });
                    let max = max.min(u32::try_from(offered).unwrap_or(u32::MAX));
                    self.pending = Pending::ChooseTargets {
                        player,
                        options,
                        player_options,
                        min,
                        max,
                        reason: TargetPrompt::Targets,
                    };
                    self.awaiting_answer = true;
                    return Ok(());
                }
            }
        }
        self.continue_loyalty_activation(player, source, ability_index, targets)
    }
}
