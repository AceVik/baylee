//! Announcing, choosing and paying for ordinary activated abilities.
#[allow(clippy::wildcard_imports)] // shared ability vocabulary
use super::*;

impl<L: CardLookup, D: crate::engine::LoopDetection> Engine<L, D> {
    /// Capture a granted ability's effects independently of its grantor. Cost
    /// choices, payment and paid-source identities use the ordinary activation path.
    pub(super) fn push_granted_ability(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        targets: SmallVec<[ObjectId; 2]>,
        granted: &crate::effects::GrantedAbility,
        grantor: Option<baylee_core::ids::DamageSourceRef>,
    ) -> ObjectId {
        let captured = self
            .activating_abilities
            .take()
            .filter(|(id, _)| *id == source);
        let object = self.state.object(source);
        let acquisition = granted.acquisition.unwrap_or_else(|| {
            captured.as_ref().map_or_else(
                || object.map_or(0, GameObject::rules_acquisition),
                |(_, list)| list.acquisition,
            )
        });
        let version = object.map(|object| object.version);
        let name = object.map_or(NameRef::new(0), |o| o.base.name);
        let card = object.and_then(|o| o.card).map(|c| c.index);
        let base = self.state.bare_base(name);
        let id = self.state.arena.insert_with(|id| {
            GameObject::new_ability_on_stack(
                id,
                player,
                AbilityLoc {
                    card,
                    index: granted.id,
                    source,
                },
                targets,
                base,
            )
        });
        if let Some(identity) = self.state.source_identity(id) {
            self.state.text_changes.set(identity, granted.text);
        }
        if let Some(object) = self.state.object_mut(id) {
            object.riders.push(crate::object::Rider::GrantedActivated);
            if let Some(grantor) = grantor {
                object
                    .riders
                    .push(crate::object::Rider::GrantedSource(grantor));
            }
            if acquisition != 0 {
                object
                    .riders
                    .push(crate::object::Rider::RulesAcquisition(acquisition));
            }
            if let Some(version) = version {
                object
                    .riders
                    .push(crate::object::Rider::AbilitySourceVersion(version));
            }
            object.target_req = match granted.definition {
                AbilityDef::Activated { targets, .. }
                | AbilityDef::ActivatedConditional { targets, .. }
                | AbilityDef::Loyalty { targets, .. } => targets,
                _ => None,
            };
        }
        self.synthetic_fx.insert(id, granted.effects);
        self.state
            .zones
            .insert(id, ZoneLocation::Stack, ZonePosition::Top, false);
        self.state.capture_linked_references(id, granted.effects);
        self.state.journal.record(GameEvent::AbilityTriggered {
            object: id,
            source,
            ability_index: granted.id,
            controller: player,
        });
        id
    }

    pub(crate) fn start_activation(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        ability_index: u32,
        targets: SmallVec<[ObjectId; 2]>,
    ) -> Result<(), EngineError> {
        self.start_activation_with_price(player, source, ability_index, targets, None)
    }

    #[allow(clippy::too_many_lines)] // activation is a staged checklist; extraction would obscure it
    pub(crate) fn start_activation_with_price(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        ability_index: u32,
        targets: SmallVec<[ObjectId; 2]>,
        locked_price: Option<(ActivationPrice, baylee_core::mana::ManaCost, u32)>,
    ) -> Result<(), EngineError> {
        // The player half of a choice this activation has already answered,
        // put here by `apply`. Read out before anything at all can return,
        // so that an activation refused further down cannot leave it
        // standing for the next one to inherit.
        let chosen_players = std::mem::take(&mut self.activation_target_players);
        // Turning a permanent face up and unlocking a door are special
        // actions (CR 116.2b, 116.2m), and a prepared cast is a cast: none
        // of the three is an activation, so none names an activator below.
        // Named before them, the activator outlived each (nothing here lets
        // go of it), and the snapshot hash and the fork read it as an
        // activation in progress.
        //
        // Prepared cast (`choice::PREPARED_CAST`): pay the linked
        // spell's cost, put a copy on the stack, unprepare the source.
        #[cfg(test)] // the engine as it was, for `activation_choice_tests`
        if super::KEEP_STALE_TARGET_CHOICE.with(std::cell::Cell::get) {
            self.activation_target_choice.controller = Some(player);
        }
        if ability_index == crate::choice::TURN_FACE_UP {
            let (mana, parts, _) = self
                .turn_face_up_spec(source)
                .ok_or(EngineError::IllegalAction("no disguise cost"))?;
            if parts.is_empty() && !mana.has_variable() {
                return self.turn_face_up(player, source);
            }
        }
        if let Some(half) = crate::choice::door_to_unlock(ability_index) {
            return self.unlock_door(player, source, half);
        }
        if ability_index == crate::choice::PLOT_CARD {
            return self.plot_card(player, source);
        }
        if ability_index == crate::choice::PREPARED_CAST {
            return self.start_prepared_cast(player, source);
        }
        // Who is activating, for a target question someone else may answer
        // (`activation_target_chooser`) and for its answer; let go of where
        // the ability reaches the stack or the activation is reversed.
        self.activation_target_choice.controller = Some(player);
        // Karn's lock, ahead of every route below it. It used to sit inside
        // the printed-ability branch, which the granted and loyalty branches
        // return before reaching — so a locked artifact's *granted* ability
        // passed under it. Structural rather than observed: the offer guard
        // above refuses first, and no test gets this far.
        if ability_index != crate::choice::TURN_FACE_UP
            && self.artifact_activations_are_locked(source)
        {
            return Err(EngineError::IllegalAction(
                "activated abilities of artifacts can't be activated (Karn)",
            ));
        }
        // A granted ability follows the same announcements, cost choices and
        // payment as a printed one (CR 602.2b). Its effects are captured below
        // before paying can remove either the recipient or the grantor.
        let granted = if crate::choice::granted_slot(ability_index).is_some() {
            let granted = self
                .granted_definition(source, ability_index)
                .ok_or(EngineError::IllegalAction("no granted ability"))?;
            self.activating_grant = Some((source, ability_index, granted));
            Some(granted)
        } else {
            None
        };
        let granted_definition = granted.map(|grant| grant.definition);
        // Loyalty abilities route to their own activation path first.
        if let Some(cost) = self.loyalty_cost(source, ability_index) {
            return self.start_loyalty_activation(player, source, ability_index, targets, cost);
        }
        if self.announce_activation_mode(player, source, ability_index)? {
            return Ok(());
        }
        let (cost, effects, (first, second), mana_ability, zone, limit, cost_reduction) = {
            let _obj = self
                .state
                .object(source)
                .ok_or(EngineError::IllegalAction("no such permanent"))?;
            // Deliberately no `obj.card` here. The list this ability was
            // offered from is the object's *own* (`GameObject::abilities`),
            // which answers for a Treasure out of its definition and for a
            // token copy out of the rules text it copied — so demanding a
            // card refused an ability the engine had just offered, which is
            // the failure mode this engine treats as worse than either half
            // of it alone. The card index it used to read was never used
            // for anything: it was discarded again further down.
            //
            // The object's own list, which is what `legal_actions` indexed
            // when it offered this. Reading the card's instead was the
            // "two probes must agree" bug once more: a Glasspool Mimic
            // copying Werefox Bodyguard was offered the sacrifice ability
            // at index 1 and refused it with "no such ability", because
            // Glasspool Mimic's printed list is one entry long.
            let ability = granted_definition
                .or_else(|| self.activation_definition(source, ability_index))
                .ok_or(EngineError::IllegalAction("no such ability"))?;
            if ability.is_intrinsic_mana_ability()
                && casting::intrinsic_mana_colors(&self.state, source).is_empty()
            {
                return Err(EngineError::IllegalAction("no basic land mana ability"));
            }
            let selected = match &ability {
                AbilityDef::ModalActivated { modes, .. } => {
                    let mode = self
                        .activation_mode_index(source, ability_index)
                        .ok_or(EngineError::IllegalAction("activation mode not chosen"))?;
                    super::modes::selected_definition(&ability, &modes[usize::from(mode)])
                }
                _ => ability,
            };
            match &selected {
                AbilityDef::Unearth { cost } | AbilityDef::Encore { cost } => (
                    *cost,
                    &[][..],
                    (None, None),
                    false,
                    ActivationZone::Graveyard,
                    ActivationLimit::Unlimited,
                    None,
                ),
                AbilityDef::Activated {
                    cost,
                    effects,
                    targets: first,
                    second_targets,
                    mana_ability,
                    zone,
                    limit,
                    cost_reduction,
                    ..
                } => (
                    *cost,
                    *effects,
                    (*first, *second_targets),
                    *mana_ability
                        && baylee_cards_dsl::mana_rule::activated_mana_ability(
                            cost,
                            effects,
                            first.is_some() || second_targets.is_some(),
                        ),
                    *zone,
                    *limit,
                    *cost_reduction,
                ),
                AbilityDef::ActivatedConditional {
                    cost,
                    effects,
                    targets: first,
                    second_targets,
                    mana_ability,
                    zone,
                    condition,
                    limit,
                    cost_reduction,
                    ..
                } => {
                    if !crate::eval::condition_holds_with_context(
                        &self.state,
                        player,
                        self.ability_rule_context(source, ability_index),
                        *condition,
                    ) {
                        return Err(EngineError::IllegalAction("activation condition not met"));
                    }
                    (
                        *cost,
                        *effects,
                        (*first, *second_targets),
                        *mana_ability
                            && baylee_cards_dsl::mana_rule::activated_mana_ability(
                                cost,
                                effects,
                                first.is_some() || second_targets.is_some(),
                            ),
                        *zone,
                        *limit,
                        *cost_reduction,
                    )
                }
                _ => return Err(EngineError::IllegalAction("not an activated ability")),
            }
        };
        // Classification belongs to the whole ability, including when the chosen
        // mode itself does not add mana.
        let mana_ability = if self.activation_mode.is_some() {
            self.activation_definition(source, ability_index)
                .is_some_and(|a| a.is_mana_ability())
        } else {
            mana_ability
        };
        if !mana_ability
            && ability_index != crate::choice::TURN_FACE_UP
            && self.non_mana_activations_are_locked(source)
        {
            return Err(EngineError::IllegalAction(
                "non-mana activated abilities of this permanent can't be activated",
            ));
        }
        // The price, determined once and before anything is paid (CR 601.2f
        // through CR 602.2b): Boseiju's channel discards the very card whose
        // count of legendary creatures lowers it, and a total cost is locked
        // in once determined. The offer asked the same helper, so the ability
        // is charged what it was offered at.

        // Read before the cost moves the card (CR 702.29c needs to know,
        // after the discard, that it paid a cycling ability's cost).
        let cycling = self.state.object(source).is_some_and(|o| {
            o.abilities(&self.lookup)
                .get(ability_index as usize)
                .is_some_and(AbilityDef::is_cycling)
        });
        // Read before any cost is paid, because a cost may move the source
        // and a moved copy is no longer one — see `Engine::activating_abilities`.
        self.activating_abilities = self.state.object(source).map(|o| {
            (
                source,
                o.ability_list(&self.lookup)
                    .with_base_text(self.state.text_changes.get(
                        baylee_core::ids::DamageSourceRef {
                            object: source,
                            version: o.version,
                        },
                    )),
            )
        });
        // Zone validation (battlefield abilities vs. hand abilities).
        let in_right_zone = match zone {
            ActivationZone::Battlefield => self
                .state
                .object(source)
                .is_some_and(|o| o.zone == Zone::Battlefield && o.controller == player),
            ActivationZone::BattlefieldOrCommand => self.state.object(source).is_some_and(|o| {
                (o.zone == Zone::Battlefield && o.controller == player)
                    || (o.zone == Zone::Command && o.owner == player && !o.is_merged_component())
            }),
            ActivationZone::BattlefieldOwner => self
                .state
                .object(source)
                .is_some_and(|o| o.zone == Zone::Battlefield && o.owner == player),
            ActivationZone::BattlefieldAnyPlayer => self
                .state
                .object(source)
                .is_some_and(|o| o.zone == Zone::Battlefield),
            ActivationZone::BattlefieldOpponent => self.state.object(source).is_some_and(|o| {
                o.zone == Zone::Battlefield && self.state.is_opponent(player, o.controller)
            }),
            ActivationZone::BattlefieldAttachedController => {
                self.state
                    .object(source)
                    .is_some_and(|o| o.zone == Zone::Battlefield)
                    && crate::eval::controller_of_attached(&self.state, source) == Some(player)
            }
            ActivationZone::Command => self.state.object(source).is_some_and(|o| {
                o.zone == Zone::Command && o.owner == player && !o.is_merged_component()
            }),
            ActivationZone::Hand => self
                .state
                .object(source)
                .is_some_and(|o| o.zone == Zone::Hand && o.zone_owner == Some(player)),
            ActivationZone::HandOrCommand => self.state.object(source).is_some_and(|o| {
                (o.zone == Zone::Hand && o.zone_owner == Some(player))
                    || (o.zone == Zone::Command && o.owner == player && !o.is_merged_component())
            }),
            ActivationZone::Graveyard => self
                .state
                .object(source)
                .is_some_and(|o| o.zone == Zone::Graveyard && o.zone_owner == Some(player)),
        };
        if !in_right_zone {
            return Err(EngineError::IllegalAction(
                "ability not usable from this zone",
            ));
        }
        let _ = zone;
        let printed_cost = cost;
        let resource_price =
            self.activation_price(player, source, ability_index, &printed_cost, cost_reduction);
        if self.announce_activation_x(player, source, ability_index, &resource_price)? {
            return Ok(());
        }
        let variable_mana = printed_cost.mana.has_variable();
        let targets_resolved = (first.is_none() || self.activation_targets_answered)
            && (second.is_none() || self.activation_second_targets.is_some());
        let chosen_so_far: SmallVec<[ObjectId; 4]> = targets
            .iter()
            .copied()
            .chain(
                self.activation_second_targets
                    .iter()
                    .flat_map(|second_ts| second_ts.iter().copied()),
            )
            .collect();
        let initial_target_pricing = if targets_resolved {
            super::costs::ActivationTargetPricing::Chosen(&chosen_so_far)
        } else {
            super::costs::ActivationTargetPricing::Preview
        };
        let had_locked_price = locked_price.is_some();
        let (mut cost, mut fixed_mana_cost, additional_life) = locked_price.unwrap_or_else(|| {
            let price = |x| {
                self.activation_price_for_targets(
                    player,
                    source,
                    ability_index,
                    &Cost {
                        mana: printed_cost.mana.with_x(x),
                        parts: printed_cost.parts,
                    },
                    cost_reduction,
                    initial_target_pricing,
                )
            };
            let selected = price(self.activation_x.unwrap_or(0));
            (
                selected,
                price(0).mana,
                if mana_ability {
                    crate::mana_rules::life_price(&self.state, source)
                } else {
                    0
                },
            )
        });
        let settled_x = self.activation_x.unwrap_or(0);
        let splice_either = |price: &mut ActivationPrice,
                             fixed: &mut baylee_core::mana::ManaCost,
                             at: usize,
                             chosen: &Cost| {
            price.mana = price.mana.combine(&chosen.mana.with_x(settled_x));
            *fixed = fixed.combine(&chosen.mana.with_x(0));
            price.parts = price.parts[..at]
                .iter()
                .chain(chosen.parts)
                .chain(&price.parts[at + 1..])
                .copied()
                .collect();
        };
        let mut applied_eithers = 0_usize;
        while applied_eithers < self.activation_either_branches.len() {
            let Some((at, CostPart::Either(a, b))) = cost
                .parts
                .iter()
                .enumerate()
                .find(|(_, p)| matches!(p, CostPart::Either(_, _)))
            else {
                break;
            };
            let chosen = if self.activation_either_branches[applied_eithers] {
                *a
            } else {
                *b
            };
            splice_either(&mut cost, &mut fixed_mana_cost, at, chosen);
            applied_eithers += 1;
        }
        while let Some((at, CostPart::Either(a, b))) = cost
            .parts
            .iter()
            .enumerate()
            .find(|(_, p)| matches!(p, CostPart::Either(_, _)))
        {
            let (first_cost, second_cost) = (*a, *b);
            let branch_ok = |sub: &Cost| {
                let mut probe = cost.clone();
                let mut probe_fixed = fixed_mana_cost;
                splice_either(&mut probe, &mut probe_fixed, at, sub);
                self.activation_affordable(player, source, &probe, &[], additional_life)
                    || ((variable_mana || sub.mana.has_variable())
                        && self.can_make_more_mana(player, source, ability_index)
                        && self.activation_affordable(
                            player,
                            source,
                            &probe.with_mana(baylee_core::mana::ManaCost::ZERO),
                            &[],
                            additional_life,
                        ))
            };
            let first_ok = branch_ok(first_cost);
            let second_ok = branch_ok(second_cost);
            let branch_idx = u8::try_from(self.activation_either_branches.len()).unwrap_or(u8::MAX);
            match (first_ok, second_ok) {
                (true, true) => {
                    self.activation_target_players = chosen_players;
                    self.pending_plan = Some(PlanKind::ChooseActivationCostBranch {
                        source,
                        ability_index,
                    });
                    self.pending = Pending::YesNo {
                        player,
                        prompt: crate::choice::YesNoPrompt::ChooseFirstCostBranch {
                            index: branch_idx,
                        },
                        source: self
                            .state
                            .printed_ability_list(source)
                            .and_then(|list| list.entry(ability_index as usize))
                            .and_then(|entry| entry.provenance.ability_ref())
                            .or_else(|| {
                                self.state.object(source).and_then(|o| o.card).map(|card| {
                                    baylee_core::ids::AbilityRef::new(card.index, ability_index)
                                })
                            }),
                    };
                    self.awaiting_answer = true;
                    return Ok(());
                }
                (true, false) => {
                    self.activation_either_branches.push(true);
                    splice_either(&mut cost, &mut fixed_mana_cost, at, first_cost);
                }
                (false, true) => {
                    self.activation_either_branches.push(false);
                    splice_either(&mut cost, &mut fixed_mana_cost, at, second_cost);
                }
                (false, false) => {
                    self.activation_either_branches.clear();
                    self.activation_phyrexian.clear();
                    self.activation_counter_removals.clear();
                    self.activation_pending_counter_object = None;
                    self.activation_x = None;
                    return Err(EngineError::IllegalAction("cannot pay the cost"));
                }
            }
        }
        // Each Phyrexian symbol: 2 life or its mana, announced beside the X
        // (CR 601.2b through CR 602.2b; CR 118.13a). Asked only where both
        // answers still leave a way to pay; where one does, it is taken
        // without asking, and where neither does the activation is refused.
        let symbols = cost.mana.phyrexian_count();
        while u32::try_from(self.activation_phyrexian.len()).unwrap_or(u32::MAX) < symbols {
            let mut with_life = self.activation_phyrexian.clone();
            with_life.push(true);
            let mut with_mana = self.activation_phyrexian.clone();
            with_mana.push(false);
            let life_ok =
                self.activation_affordable(player, source, &cost, &with_life, additional_life);
            let mana_ok =
                self.activation_affordable(player, source, &cost, &with_mana, additional_life);
            match (life_ok, mana_ok) {
                (true, true) => {
                    self.pending_plan = Some(PlanKind::ChoosePhyrexianLife {
                        source,
                        ability_index,
                    });
                    self.pending = Pending::YesNo {
                        player,
                        prompt: crate::choice::YesNoPrompt::PayLife {
                            amount: 2,
                            permanent: None,
                        },
                        // The ability being activated, which is what the
                        // question is about.
                        source: self
                            .state
                            .printed_ability_list(source)
                            .and_then(|list| list.entry(ability_index as usize))
                            .and_then(|entry| entry.provenance.ability_ref())
                            .or_else(|| {
                                self.state.object(source).and_then(|o| o.card).map(|card| {
                                    baylee_core::ids::AbilityRef::new(card.index, ability_index)
                                })
                            }),
                    };
                    self.awaiting_answer = true;
                    return Ok(());
                }
                (true, false) => self.activation_phyrexian.push(true),
                (false, true) => self.activation_phyrexian.push(false),
                (false, false) => {
                    self.activation_either_branches.clear();
                    self.activation_phyrexian.clear();
                    self.activation_counter_removals.clear();
                    self.activation_pending_counter_object = None;
                    self.activation_x = None;
                    return Err(EngineError::IllegalAction("cannot pay the cost"));
                }
            }
        }
        let by_life = self
            .activation_phyrexian
            .iter()
            .enumerate()
            .fold(0u32, |mask, (i, life)| mask | (u32::from(*life) << i));
        let phyrexian_life = 2 * i32::try_from(by_life.count_ones()).unwrap_or(i32::MAX);
        let activation_life = i32::try_from(additional_life)
            .ok()
            .and_then(|life| life.checked_add(phyrexian_life))
            .ok_or(EngineError::IllegalAction(
                "activation life cost exceeds available life",
            ))?;
        let mut cost = cost.with_mana(cost.mana.with_phyrexian_settled(by_life));
        // Targets, unless this activation has already answered them. That is
        // a flag and not a look at the lists, because an empty list is an
        // answer too: "up to one" answered with nothing re-enters here with
        // no objects and no players (CR 115.6). Reading the lists asked the
        // same question again forever. Before that it sent an ability whose
        // targets are all players back to the same question, because only
        // the objects were read.
        //
        // This used to be written twice — once here and once after the cost
        // was paid, identically — and the second copy was unreachable:
        // nothing between them touches `targets` or `target`. What it left
        // behind is a comment claiming the engine chooses targets after
        // paying, which is neither what it does nor what the rules say
        // (CR 601.2c chooses targets, CR 601.2h pays; an activation follows
        // the same order by CR 602.2b).
        if !self.activation_targets_answered
            && let Some(req) = first
        {
            // The count is read, and is no longer assumed to be one: "tap
            // two target lands" is two, and "up to one" is none or one
            // (CR 601.2c, by CR 602.2b). X is answered by now (CR 601.2b
            // comes first), so an X count is a number here.
            let (min, mut max) = req.bounds(self.activation_x.unwrap_or(0));
            if let Some((_, total)) = crate::engine::division::effects_divided_spec(
                effects,
                &self.state,
                player,
                self.ability_rule_context(source, ability_index),
                self.state.object(source).map(|o| o.version),
                self.activation_x.unwrap_or(0),
            ) {
                max = max.min(total);
            }
            if min > max {
                self.activation_x = None;
                return Err(EngineError::IllegalAction("no legal targets"));
            }
            let mut options = eval::target_options_in_domain(
                &req.spec,
                &self.state,
                player,
                self.ability_rule_context(source, ability_index),
                self.activation_x,
                eval::target_domain::walls(first, second),
            );
            if req.constraint() == Some(baylee_cards_dsl::TargetConstraint::OnePerPlayer) {
                if !eval::one_per_player_satisfied(
                    &self.state,
                    &req.spec,
                    player,
                    &options,
                    req.min,
                ) {
                    self.activation_x = None;
                    return Err(EngineError::IllegalAction("no legal targets"));
                }
                if max == 0 {
                    self.activation_targets_answered = true;
                } else {
                    let plan = PlanKind::ActivateAbility {
                        source,
                        ability_index,
                    };
                    let Some(chooser) =
                        self.activation_target_chooser(req, player, plan.clone())?
                    else {
                        return Ok(());
                    };
                    let seats = eval::one_per_player_seats(&self.state, &req.spec, player);
                    let asking = super::super::PerOpponent::one_per_player(
                        req.spec, seats, req.min, chooser,
                    );
                    if let Some(done) = self.ask_next_activation_per_player(
                        player,
                        self.ability_rule_context(source, ability_index),
                        self.activation_x,
                        eval::target_domain::walls(first, second),
                        asking,
                        plan,
                    ) {
                        self.activation_target_references.first =
                            self.state.capture_target_group(&done.gathered);
                        self.activation_target_choice.first_groups = done.groups;
                        self.activation_targets_answered = true;
                        return self.start_activation(player, source, ability_index, done.gathered);
                    }
                    return Ok(());
                }
            } else {
                // "Target cards from a single graveyard" (Unlicensed Hearse) and
                // `TargetConstraint::SameController` ("target creatures controlled
                // by the same player"): when multiple controller/graveyard groups
                // satisfy `min`, ask which group first and narrow `options` to it.
                if req.constraint() == Some(baylee_cards_dsl::TargetConstraint::SameController)
                    || matches!(
                        req.spec,
                        baylee_cards_dsl::TargetSpec::CardInGraveyard(
                            _,
                            baylee_cards_dsl::PlayerRel::Chosen
                        )
                    )
                {
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
                                let plan = PlanKind::ChooseActivationGraveyard {
                                    source,
                                    ability_index,
                                };
                                let Some(chooser) =
                                    self.activation_target_chooser(req, player, plan.clone())?
                                else {
                                    return Ok(());
                                };
                                self.pending_plan = Some(plan);
                                self.pending = Pending::ChoosePlayer {
                                    player: chooser,
                                    options: groups,
                                };
                                self.awaiting_answer = true;
                                return Ok(());
                            }
                        }
                    }
                }
                // Players are the other half of the same choice, and asking for
                // objects alone made three implemented lands dead: Nephalia
                // Drownyard, Duskmantle and Orzhova all say "target player",
                // whose object list is empty by construction — so each was
                // offered in `LegalActions` (which does add the two counts) and
                // then refused here with "no legal targets", the two-probes
                // disagreement this engine treats as the worst kind.
                let player_options = eval::target_player_options_from(
                    &self.state,
                    &req.spec,
                    player,
                    self.ability_rule_context(source, ability_index),
                );
                if options.len() + player_options.len() < min as usize {
                    // The number was answered before this question was asked, so
                    // an activation that dies here has one to throw away.
                    self.activation_x = None;
                    return Err(EngineError::IllegalAction("no legal targets"));
                }
                if max == 0 || (options.is_empty() && player_options.is_empty()) {
                    // The only answer is none at all, which is a legal one
                    // (CR 115.6), so nobody is asked for it. The cast wizard
                    // skips the same question the same way.
                    self.activation_targets_answered = true;
                } else {
                    let plan = PlanKind::ActivateAbility {
                        source,
                        ability_index,
                    };
                    let Some(chooser) =
                        self.activation_target_chooser(req, player, plan.clone())?
                    else {
                        return Ok(());
                    };
                    self.pending_plan = Some(plan);
                    let max = max.min(
                        u32::try_from(options.len() + player_options.len()).unwrap_or(u32::MAX),
                    );
                    self.pending = Pending::ChooseTargets {
                        player: chooser,
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
        // The second instance of the word "target" (Contested Cliffs), asked
        // once the first has its answer and before anything is paid — both
        // are CR 601.2c, and the cost is CR 601.2h. Its options leave in what
        // the first instance chose, because CR 115.3 lets one object be
        // chosen once for each instance.
        if let Some(req) = second
            && self.activation_second_targets.is_none()
        {
            // A requirement naming the first target (Gauntlets of Chaos:
            // "that shares one of those types with it") reads it here.
            let mut context = self.ability_rule_context(source, ability_index);
            context.reference = targets.first().copied();
            let (min, max) = req.bounds(self.activation_x.unwrap_or(0));
            let mut options = eval::target_options_in_domain(
                &req.spec,
                &self.state,
                player,
                context,
                self.activation_x,
                eval::target_domain::walls(first, second),
            );
            if req.constraint() == Some(baylee_cards_dsl::TargetConstraint::OnePerPlayer) {
                if !eval::one_per_player_satisfied(
                    &self.state,
                    &req.spec,
                    player,
                    &options,
                    req.min,
                ) {
                    self.activation_x = None;
                    return Err(EngineError::IllegalAction("no legal targets"));
                }
                if max == 0 {
                    self.activation_second_targets = Some(SmallVec::new());
                } else {
                    let plan = PlanKind::ActivateAbilitySecondTargets {
                        source,
                        ability_index,
                        targets: targets.clone(),
                        target_players: chosen_players.clone(),
                    };
                    let Some(chooser) =
                        self.activation_target_chooser(req, player, plan.clone())?
                    else {
                        return Ok(());
                    };
                    let seats = eval::one_per_player_seats(&self.state, &req.spec, player);
                    let asking = super::super::PerOpponent::one_per_player(
                        req.spec, seats, req.min, chooser,
                    );
                    if let Some(done) = self.ask_next_activation_per_player(
                        player,
                        context,
                        self.activation_x,
                        eval::target_domain::walls(first, second),
                        asking,
                        plan,
                    ) {
                        self.activation_target_references.second =
                            self.state.capture_target_group(&done.gathered);
                        self.activation_target_choice.second_groups = done.groups;
                        self.activation_second_targets = Some(done.gathered.into_iter().collect());
                    } else {
                        return Ok(());
                    }
                }
            } else {
                if req.constraint() == Some(baylee_cards_dsl::TargetConstraint::SameController)
                    || matches!(
                        req.spec,
                        baylee_cards_dsl::TargetSpec::CardInGraveyard(
                            _,
                            baylee_cards_dsl::PlayerRel::Chosen
                        )
                    )
                {
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
                                let plan = PlanKind::ChooseActivationSecondGroup {
                                    source,
                                    ability_index,
                                    targets,
                                    target_players: chosen_players,
                                };
                                let Some(chooser) =
                                    self.activation_target_chooser(req, player, plan.clone())?
                                else {
                                    return Ok(());
                                };
                                self.pending_plan = Some(plan);
                                self.pending = Pending::ChoosePlayer {
                                    player: chooser,
                                    options: groups,
                                };
                                self.awaiting_answer = true;
                                return Ok(());
                            }
                        }
                    }
                }
                let player_options = eval::target_player_options_from(
                    &self.state,
                    &req.spec,
                    player,
                    self.ability_rule_context(source, ability_index),
                );
                if options.len() + player_options.len() < min as usize {
                    self.activation_x = None;
                    return Err(EngineError::IllegalAction("no legal targets"));
                }
                if (options.is_empty() && player_options.is_empty()) || max == 0 {
                    self.activation_second_targets = Some(SmallVec::new());
                } else {
                    let plan = PlanKind::ActivateAbilitySecondTargets {
                        source,
                        ability_index,
                        targets: targets.clone(),
                        target_players: chosen_players.clone(),
                    };
                    let Some(chooser) =
                        self.activation_target_chooser(req, player, plan.clone())?
                    else {
                        return Ok(());
                    };
                    self.pending_plan = Some(plan);
                    let max = max.min(
                        u32::try_from(options.len() + player_options.len()).unwrap_or(u32::MAX),
                    );
                    self.pending = Pending::ChooseTargets {
                        player: chooser,
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
        if self.ask_activation_division(
            player,
            source,
            ability_index,
            effects,
            targets.clone(),
            chosen_players.clone(),
        ) {
            return Ok(());
        }
        // If targets were auto-resolved on this pass (e.g. `max == 0` or empty
        // options), finalize target-dependent cost reductions against the actual
        // chosen targets instead of keeping the pre-target optimistic preview.
        let has_target_reduction = self.has_target_conditional_ability_reduction(player, source);
        if !had_locked_price && !targets_resolved && has_target_reduction {
            let final_targets: SmallVec<[ObjectId; 4]> = targets
                .iter()
                .copied()
                .chain(
                    self.activation_second_targets
                        .iter()
                        .flat_map(|second_ts| second_ts.iter().copied()),
                )
                .collect();
            let final_pricing = super::costs::ActivationTargetPricing::Chosen(&final_targets);
            let mut repriced = self.activation_price_for_targets(
                player,
                source,
                ability_index,
                &Cost {
                    mana: printed_cost.mana.with_x(settled_x),
                    parts: printed_cost.parts,
                },
                cost_reduction,
                final_pricing,
            );
            let mut repriced_fixed = self
                .activation_price_for_targets(
                    player,
                    source,
                    ability_index,
                    &Cost {
                        mana: printed_cost.mana.with_x(0),
                        parts: printed_cost.parts,
                    },
                    cost_reduction,
                    final_pricing,
                )
                .mana;
            for &take_first in &self.activation_either_branches {
                let Some((at, CostPart::Either(a, b))) = repriced
                    .parts
                    .iter()
                    .enumerate()
                    .find(|(_, p)| matches!(p, CostPart::Either(_, _)))
                else {
                    break;
                };
                let chosen = if take_first { *a } else { *b };
                splice_either(&mut repriced, &mut repriced_fixed, at, chosen);
            }
            cost = repriced.with_mana(repriced.mana.with_phyrexian_settled(by_life));
            fixed_mana_cost = repriced_fixed;
        }
        // The mana is the payment window's question when this seat can still
        // make some (CR 601.2g, by 602.2b): only the rest is asked here, and
        // the window below is opened for the mana the pool does not cover.
        let deferred = (variable_mana || has_target_reduction)
            && !self.can_pay_activation_price(
                player,
                source,
                &cost.with_mana(cost.mana.with_x(self.activation_x.unwrap_or(0))),
            )
            && self.can_make_more_mana(player, source, ability_index);
        let planned = if deferred {
            cost.with_mana(baylee_core::mana::ManaCost::ZERO)
        } else {
            cost.clone()
        };
        if !self.can_plan_activation(player, source, &planned, activation_life)
            || !self.state.can_pay_life(player, activation_life)
        {
            self.activation_cost_choices.clear();
            self.activation_counter_removals.clear();
            self.activation_pending_counter_object = None;
            self.activation_x = None;
            self.activation_phyrexian.clear();
            self.activation_either_branches.clear();
            return Err(EngineError::IllegalAction("cannot pay the cost"));
        }
        // CR 601.2h, and the one step of it the player has to take: a cost
        // that says "sacrifice a creature" names no creature. Asked after
        // `can_afford` and never before it, so nobody is made to choose what
        // to give up for an activation that cannot happen — and asked one
        // part at a time, because a cost with two asking parts is two
        // questions and answering them together would lose which answer
        // belongs to which.
        let priced_cost = cost.parts.as_slice();
        let wanted = cost_wizard::answers_wanted(priced_cost);
        // X = 0 still supplies one (empty) answer for its cost part.
        while self.activation_cost_choices.len() < wanted
            && match cost_wizard::asking_parts(priced_cost).nth(self.activation_cost_choices.len())
            {
                Some(
                    CostPart::SacrificeX(_)
                    | CostPart::TapX(_)
                    | CostPart::DiscardX(_)
                    | CostPart::ExileFromGraveyardX(_),
                ) => self.activation_x == Some(0),
                Some(CostPart::CraftMaterials { min, max, .. }) => {
                    *max == 0
                        || (*min == 0
                            && cost_wizard::menu(
                                &self.state,
                                player,
                                source,
                                priced_cost,
                                cost_wizard::asking_parts(priced_cost)
                                    .nth(self.activation_cost_choices.len())
                                    .expect("checked"),
                            )
                            .is_empty())
                }
                Some(CostPart::CollectEvidence(0)) => true,
                _ => false,
            }
        {
            self.activation_cost_choices.push(Vec::new());
        }
        if self.activation_cost_choices.len() < wanted {
            let asked = self.activation_cost_choices.len();
            let part = cost_wizard::asking_parts(priced_cost)
                .nth(asked)
                .expect("fewer answers in hand than the cost has asking parts");
            let mut options = cost_wizard::menu(&self.state, player, source, priced_cost, part);
            // Nothing is paid until every question has an answer (CR 601.2h
            // pays the whole cost at once), so the board the second question
            // is asked of still holds the object the first one named. Taking
            // the answers so far off the menu is what stops a cost with two
            // sacrifice parts from eating one permanent twice. No card in
            // the pool prints such a cost today; this is one line and is
            // right for the one that does.
            options.retain(|id| {
                !self
                    .activation_cost_choices
                    .iter()
                    .any(|answer| answer.contains(id))
            });
            if matches!(part, CostPart::Forage) {
                cost_wizard::prune_incomplete_forage_graveyard(&self.state, &mut options);
            }
            let prompt = cost_wizard::prompt(part);
            // `can_afford` asked the same question of the same function a
            // moment ago, so an empty list here is not a board this engine
            // can be in. Refusing rather than asserting all the same: this
            // is a rules path, and one process per game is the panic
            // boundary.
            if options.is_empty()
                && !matches!(
                    part,
                    CostPart::Crew(0) | CostPart::Saddle(0) | CostPart::CollectEvidence(0)
                )
            {
                self.activation_cost_choices.clear();
                self.activation_x = None;
                return Err(EngineError::IllegalAction("nothing can pay this cost"));
            }
            self.pending_plan = Some(PlanKind::PayActivationCost {
                source,
                ability_index,
                targets,
                target_players: chosen_players,
                remaining: match part {
                    CostPart::TapN { count, .. } => Some(*count),
                    CostPart::ExileFromSingleGraveyard { count, .. } => Some(u32::from(*count)),
                    CostPart::SacrificeX(_)
                    | CostPart::TapX(_)
                    | CostPart::DiscardX(_)
                    | CostPart::ExileFromGraveyardX(_) => Some(self.activation_x.unwrap_or(0)),
                    CostPart::Forage => Some(3),
                    _ => None,
                },
                chosen: Vec::new(),
            });
            // One object per question, except crew's, whose one question
            // is answered with any number of creatures with total power N or
            // greater (CR 702.122a). The total is stated with each
            // creature's power beside it, so an answer short of it is
            // refused for a reason the question gives, and the offer above
            // (`can_afford`) asked the same sum of the same menu.
            let (max, total) = if let CostPart::Crew(power) | CostPart::Saddle(power) = part {
                let payment = cost_wizard::PowerPayment::new(&self.state, source, part);
                let weights = options.iter().map(|id| payment.weight(*id)).collect();
                (
                    u8::try_from(options.len()).unwrap_or(u8::MAX),
                    Some(crate::choice::CardTotal {
                        of: crate::choice::Measure::Power,
                        weights,
                        at_least: Some(i32::from(*power)),
                        at_most: None,
                    }),
                )
            } else if let CostPart::CollectEvidence(mv) = part {
                let weights = options
                    .iter()
                    .map(|&id| {
                        self.state.object(id).map_or(0, |o| {
                            i32::try_from(o.characteristics().mana_value()).unwrap_or(i32::MAX)
                        })
                    })
                    .collect();
                (
                    u8::try_from(options.len()).unwrap_or(u8::MAX),
                    Some(crate::choice::CardTotal {
                        of: crate::choice::Measure::ManaValue,
                        weights,
                        at_least: Some(i32::from(*mv)),
                        at_most: None,
                    }),
                )
            } else if let CostPart::TapN { count, .. } = part {
                (u8::try_from(*count).unwrap_or(u8::MAX), None)
            } else if let CostPart::ExileFromSingleGraveyard { count, .. } = part {
                // A batch cannot state "all from one graveyard" when several
                // graveyards are offered. The first card chooses the graveyard;
                // PayActivationCost then narrows the remaining card menu to it.
                // Keep the existing batch when only one graveyard can pay.
                let first_owner = options
                    .first()
                    .and_then(|id| self.state.object(*id))
                    .and_then(|object| object.zone_owner);
                let several_graveyards = options.iter().any(|id| {
                    self.state.object(*id).and_then(|object| object.zone_owner) != first_owner
                });
                (
                    if several_graveyards {
                        (*count).min(1)
                    } else {
                        *count
                    },
                    None,
                )
            } else if matches!(
                part,
                CostPart::SacrificeX(_)
                    | CostPart::TapX(_)
                    | CostPart::DiscardX(_)
                    | CostPart::ExileFromGraveyardX(_)
            ) {
                (
                    u8::try_from(self.activation_x.unwrap_or(0)).unwrap_or(u8::MAX),
                    None,
                )
            } else if let CostPart::CraftMaterials { max, .. } = part {
                (
                    (*max).min(u8::try_from(options.len()).unwrap_or(u8::MAX)),
                    None,
                )
            } else {
                (1, None)
            };
            self.pending = Pending::ChooseCards {
                player,
                options,
                min: if let CostPart::CraftMaterials { min, .. } = part {
                    *min
                } else if matches!(
                    part,
                    CostPart::SacrificeX(_)
                        | CostPart::TapX(_)
                        | CostPart::DiscardX(_)
                        | CostPart::ExileFromGraveyardX(_)
                        | CostPart::TapN { .. }
                        | CostPart::ExileFromSingleGraveyard { .. }
                ) {
                    max
                } else {
                    u8::from(!matches!(
                        part,
                        CostPart::Crew(0) | CostPart::Saddle(0) | CostPart::CollectEvidence(0)
                    ))
                },
                max,
                prompt,
                total,
            };
            self.awaiting_answer = true;
            return Ok(());
        }
        let excluded_for_counters: SmallVec<[ObjectId; 4]> = cost_wizard::asking_parts(priced_cost)
            .zip(self.activation_cost_choices.iter())
            .filter(|(part, _)| {
                matches!(
                    part,
                    CostPart::Sacrifice(_)
                        | CostPart::SacrificeX(_)
                        | CostPart::ExilePermanent(_)
                        | CostPart::ReturnToHand(_)
                        | CostPart::ReturnUnblockedAttacker
                        | CostPart::Forage
                        | CostPart::CraftMaterials { .. }
                )
            })
            .flat_map(|(_, ans)| ans.iter().copied())
            .filter(|&id| {
                self.state
                    .object(id)
                    .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield)
            })
            .collect();
        let counter_ctx = self.ability_rule_context(source, ability_index);
        loop {
            if let Some(chosen_obj) = self.activation_pending_counter_object {
                let kinds = cost_wizard::counter_remove::valid_kinds_for_chosen_object(
                    &self.state,
                    player,
                    counter_ctx,
                    (priced_cost, settled_x),
                    &excluded_for_counters,
                    &self.activation_counter_removals,
                    chosen_obj,
                );
                match kinds.as_slice() {
                    [] => {
                        self.activation_cost_choices.clear();
                        self.activation_counter_removals.clear();
                        self.activation_pending_counter_object = None;
                        self.activation_x = None;
                        return Err(EngineError::IllegalAction("nothing can pay this cost"));
                    }
                    [only] => {
                        self.activation_pending_counter_object = None;
                        self.activation_counter_removals.push((chosen_obj, *only));
                        continue;
                    }
                    _ => {
                        self.pending_plan = Some(PlanKind::PayActivationCounterKind {
                            source,
                            ability_index,
                            targets,
                            target_players: chosen_players,
                        });
                        self.pending = Pending::ChooseCastMode {
                            player,
                            object: chosen_obj,
                            options: kinds
                                .into_iter()
                                .enumerate()
                                .map(|(index, ck)| crate::choice::CastModeDesc {
                                    index: u32::try_from(index).unwrap_or(u32::MAX),
                                    kind: crate::choice::CastModeKind::Counter(ck),
                                    cost: baylee_core::mana::ManaCost::ZERO,
                                })
                                .collect(),
                        };
                        self.awaiting_answer = true;
                        return Ok(());
                    }
                }
            }
            match cost_wizard::counter_remove::next_counter_step(
                &self.state,
                player,
                counter_ctx,
                priced_cost,
                settled_x,
                &excluded_for_counters,
                &self.activation_counter_removals,
            ) {
                cost_wizard::counter_remove::CounterStep::Complete => break,
                cost_wizard::counter_remove::CounterStep::Impossible => {
                    self.activation_cost_choices.clear();
                    self.activation_counter_removals.clear();
                    self.activation_pending_counter_object = None;
                    self.activation_x = None;
                    return Err(EngineError::IllegalAction("nothing can pay this cost"));
                }
                cost_wizard::counter_remove::CounterStep::AutoPick(obj, ck) => {
                    self.activation_counter_removals.push((obj, ck));
                }
                cost_wizard::counter_remove::CounterStep::AskKind { object, .. } => {
                    self.activation_pending_counter_object = Some(object);
                }
                cost_wizard::counter_remove::CounterStep::AskObject { options } => {
                    self.pending_plan = Some(PlanKind::PayActivationCounterObject {
                        source,
                        ability_index,
                        targets,
                        target_players: chosen_players,
                    });
                    self.pending = Pending::ChooseCards {
                        player,
                        options,
                        min: 1,
                        max: 1,
                        prompt: crate::choice::ChoicePrompt::CostRemoveCounter,
                        total: None,
                    };
                    self.awaiting_answer = true;
                    return Ok(());
                }
            }
        }
        if !self.can_pay_activation_price(
            player,
            source,
            &cost.with_mana(cost.mana.with_x(self.activation_x.unwrap_or(0))),
        ) {
            self.activation_target_players = chosen_players;
            return self.open_activation_payment(
                player,
                source,
                ability_index,
                targets,
                (
                    cost.with_mana(cost.mana.with_x(self.activation_x.unwrap_or(0))),
                    fixed_mana_cost,
                    additional_life,
                ),
            );
        }
        let answers = std::mem::take(&mut self.activation_cost_choices);
        let counter_removals = std::mem::take(&mut self.activation_counter_removals);
        let division = std::mem::take(&mut self.activation_division);
        self.activation_pending_counter_object = None;
        // Taken rather than read, for `activation_cost_choices`' reason one
        // line up: the number belongs to this activation and to no other.
        let x = self.activation_x.take().unwrap_or(0);
        self.activation_targets_answered = false;
        self.activation_phyrexian.clear();
        self.activation_either_branches.clear();
        let activated_source_version = self.state.object(source).map(|o| o.version);
        let activated_text = granted.map_or_else(
            || {
                self.activating_abilities.as_ref().map_or(
                    self.state.ability_text(source, ability_index),
                    |(_, list)| list.base_text(ability_index as usize),
                )
            },
            |grant| grant.text,
        );
        let circled_colors = if granted.is_some() {
            // An ability-only grant does not copy the donor's deck-building choices.
            baylee_core::color::ColorSet::EMPTY
        } else {
            self.activating_abilities.as_ref().map_or_else(
                || {
                    self.state
                        .printed_ability_list(source)
                        .map_or(baylee_core::color::ColorSet::EMPTY, |list| {
                            list.circled_colors(ability_index as usize)
                        })
                },
                |(_, list)| list.circled_colors(ability_index as usize),
            )
        };
        let acquisition = granted
            .and_then(|grant| grant.acquisition)
            .unwrap_or_else(|| {
                self.activating_abilities.as_ref().map_or_else(
                    || {
                        self.state
                            .object(source)
                            .map_or(0, GameObject::rules_acquisition)
                    },
                    |(_, list)| list.acquisition,
                )
            });
        // What the source is attached to as it is activated: a cost that
        // sacrifices it (Thrull Retainer) moves it before the ability is on
        // the stack to remember it (CR 113.7a, CR 608.2h).
        let activated_source_attachment = self
            .state
            .object(source)
            .filter(|o| o.zone == crate::zone::Zone::Battlefield)
            .and_then(|o| o.attached_to)
            .and_then(|host| self.state.object(host).map(|h| (host, h.version)));
        let completion = super::payment::ActivationCompletion {
            player,
            source,
            ability_index,
            targets,
            chosen_players,
            division,
            cost: cost.clone(),
            x,
            fixed_mana_cost,
            activation_life,
            activated_source_version,
            activated_source_attachment,
            activated_text,
            circled_colors,
            acquisition,
            cycling,
            limit,
            effects,
            mana_ability,
            granted,
            second,
            first,
            mode_index: self.activation_mode.take().map(|mode| mode.mode),
        };
        let mut work = super::payment::CostPayment::new(
            &self.state,
            player,
            source,
            cost,
            answers,
            counter_removals,
            x,
        );
        if !self.begin_cost_payment(&mut work)? {
            self.activation_payment = Some(super::payment::ActivationPayment::new(
                super::payment::ActivationPaymentKind::Ordinary(Box::new(
                    super::payment::OrdinaryPayment { work, completion },
                )),
            ));
            self.offer_counter_order();
            return Ok(());
        }
        self.complete_paid_activation(completion, work.paid);
        Ok(())
    }

    /// Lets go of what an activation captured for its push: the source's
    /// ability list ([`Engine::activating_abilities`]) and a granted
    /// ability's definition (`activating_grant`).
    ///
    /// The push takes them; this is for the two other ways an activation
    /// ends. A mana ability goes on no stack and resolves at once
    /// (CR 605.3b), and an activation refused after an answer is reversed
    /// (`reverse_activation`). Left standing, the capture outlived its
    /// activation: the settled priority after it was refused a fork as an
    /// activation in progress, and the next reads of the same source (a
    /// trigger's push, the offer's words and walls for its activated
    /// abilities) took the list as it was then rather than as it is.
    ///
    /// [`Engine::activating_abilities`]: crate::engine::Engine
    pub(in crate::engine) fn release_activation_capture(&mut self) {
        #[cfg(test)]
        if super::KEEP_STALE_CAPTURE.with(std::cell::Cell::get) {
            return;
        }
        self.activating_abilities = None;
        self.activating_grant = None;
    }

    /// Lets go of the activator and the seats its target question named
    /// (`activation_target_choice`), once a loyalty ability is on the stack.
    ///
    /// `complete_paid_activation` does the same at its end for every other
    /// activation. The loyalty path finishes in its own function, which
    /// pushed the ability and kept the activator: the field is in the
    /// snapshot hash, so every hash after a planeswalker's ability carried
    /// it until the next activation was pressed, and the fork refused every
    /// position in between as an activation in progress.
    pub(in crate::engine) fn release_target_choice(&mut self) {
        #[cfg(test)]
        if super::KEEP_STALE_TARGET_CHOICE.with(std::cell::Cell::get) {
            return;
        }
        self.activation_target_choice = super::super::targeting::ActivationTargetChoice::default();
    }

    pub(super) fn record_activation(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        version: Option<u32>,
        cost: &ActivationPrice,
        ability: Option<ObjectId>,
    ) {
        self.state.journal.record(GameEvent::AbilityActivated {
            source: baylee_core::ids::DamageSourceRef {
                object: source,
                version: version.expect("activated source identity"),
            },
            ability: ability.and_then(|id| self.state.source_identity(id)),
            player,
            tap_cost: cost.parts.contains(&baylee_cards_dsl::CostPart::TapSelf),
            mana_ability: ability.is_none(),
        });
        // Capture before an immediate mana instruction changes its observers.
        self.capture_cost_triggers();
    }
}
